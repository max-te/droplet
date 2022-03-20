use axum::error_handling::HandleErrorLayer;
use axum::{extract::BodyStream, http::StatusCode, routing::patch, Server};

use async_tar::Archive;
use axum::{extract::Extension, Router};
use futures_util::stream::TryStreamExt;
use std::path::PathBuf;
use std::{env, time::Duration};
use tokio::fs::create_dir_all;
use tower::limit::concurrency::ConcurrencyLimitLayer;
use tower::timeout::TimeoutLayer;
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::TraceLayer;
use tracing::{error, info};

fn convert_error(input: axum::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, input.to_string())
}

#[tracing::instrument]
async fn extract_tar_body(target_dir: Extension<PathBuf>, stream: BodyStream) -> StatusCode {
    let reader = stream.map_err(convert_error).into_async_read();
    let ar = Archive::new(reader);
    match ar.unpack(target_dir.as_path()).await {
        Ok(()) => {
            info!("PATCH applied.");
            StatusCode::ACCEPTED
        }
        Err(err) => {
            error!(%err, "Failed PATCHing.");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

#[tracing::instrument]
async fn remove_contents_all(path: &std::path::Path) -> std::io::Result<()> {
    let mut entries = tokio::fs::read_dir(path).await?;
    while let Some(child) = entries.next_entry().await? {
        tokio::fs::remove_dir_all(child.path()).await?;
    }
    Ok(())
}

#[tracing::instrument]
async fn clear_dir_then_extract(target_dir: Extension<PathBuf>, stream: BodyStream) -> StatusCode {
    info!("PUT: Clearing directory for subseqent patching.");
    if let Err(err) = remove_contents_all(target_dir.as_path()).await {
        error!(%err, ?target_dir, "Failed clearing directory.");
        StatusCode::INTERNAL_SERVER_ERROR
    } else {
        extract_tar_body(target_dir, stream).await
    }
}

#[tracing::instrument]
async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[tracing::instrument]
async fn run_server() {
    let addr_str = env::var("DROPLET_ADDRESS").unwrap_or_else(|_| String::from("0.0.0.0:3000"));
    let addr = addr_str
        .parse()
        .expect("Could not parse socket address {addr_str}.");

    let target_dir = env::var_os("DROPLET_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"droplet_out"));

    create_dir_all(target_dir.as_path())
        .await
        .expect("Could not create target directory {target_dir:?}");

    let mut app = Router::new()
        .fallback(patch(extract_tar_body).put(clear_dir_then_extract))
        .layer(Extension(target_dir.clone()))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(HandleErrorLayer::new(|_: BoxError| async {
                    StatusCode::REQUEST_TIMEOUT
                }))
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(ConcurrencyLimitLayer::new(1)),
        );

    if let Ok(token) = env::var("DROPLET_AUTH_BEARER") {
        app = app.layer(tower_http::auth::RequireAuthorizationLayer::bearer(&token));
    }

    let server = Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal());

    info!(
        socket = ?addr,
        target_dir = ?target_dir,
        "droplet is listening."
    );
    if let Err(err) = server.await {
        error!(%err, "Server error");
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt::init();

    run_server().await
}
