use axum::{extract::BodyStream, http::StatusCode, routing::patch, Server};

use async_tar::Archive;
use axum::{extract::Extension, Router};
use futures_util::stream::TryStreamExt;
use std::env;
use std::path::PathBuf;
use tokio::fs::create_dir_all;

fn convert_error(input: axum::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, input.to_string())
}

async fn extract_tar_body(target_dir: Extension<PathBuf>, stream: BodyStream) -> StatusCode {
    println!("Receiving PATCH");
    let reader = stream.map_err(convert_error).into_async_read();
    let ar = Archive::new(reader);
    match ar.unpack(target_dir.as_path()).await {
        Ok(_) => {
            println!("PATCH applied");
            StatusCode::ACCEPTED
        }
        Err(err) => {
            println!("Error PATCHing: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let addr_str = env::var("DROPLET_ADDRESS").unwrap_or_else(|_| String::from("0.0.0.0:3000"));
    let addr = addr_str.parse().expect("Could not parse socket address.");

    let dir = env::var_os("DROPLET_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"."));
    create_dir_all(dir.as_path())
        .await
        .unwrap_or_else(|_| panic!("Could not create target directory {}", dir.display()));

    let app = Router::new()
        .fallback(patch(extract_tar_body))
        .layer(Extension(dir.clone()));
    let server = Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal());

    println!(
        "Droplet listening on {} with target {}",
        addr,
        dir.display()
    );
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
