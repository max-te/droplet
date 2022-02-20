use std::convert::Infallible;
use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Method, StatusCode};
use async_tar::Archive;
use futures_util::stream::{TryStreamExt};
use std::env;
use std::path::Path;
use tokio::fs::create_dir_all;


fn hpyertostdioerror(input: hyper::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, input.message().to_string())
}

fn get_target_dir() -> String {
    env::var("DROPLET_TARGET_DIR").unwrap_or(String::from("."))
}


async fn extract_tar_body(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut response = Response::new(Body::empty());

    match req.method() {
        &Method::PATCH => {
            println!("Receiving PATCH");
            let reader = req.into_body().into_stream().map_err(hpyertostdioerror).into_async_read();
            let ar= Archive::new(reader);
            match ar.unpack(Path::new(&get_target_dir())).await {
                Ok(_) => {
                    *response.status_mut() = StatusCode::ACCEPTED;
                    println!("PATCH applied");
                }
                Err(err) => {
                    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    println!("Error PATCHing: {}", err);
                }
            }
        },
        _ => {
            *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
        },
    };

    Ok(response)
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}


#[tokio::main]
async fn main() {
    // We'll bind to 127.0.0.1:3000
    let addr_str = env::var("DROPLET_ADDRESS").unwrap_or(String::from("0.0.0.0:3000"));
    let addr = addr_str.parse().expect("Could not parse socket address.");

    let dir = get_target_dir();
    create_dir_all(Path::new(&dir)).await.expect(&format!("Could not create target directory {}", dir));


    // A `Service` is needed for every connection, so this
    // creates one from our `extract_tar_body` function.
    let make_svc = make_service_fn(|_conn| async {
        // service_fn converts our function into a `Service`
        Ok::<_, Infallible>(service_fn(extract_tar_body))
    });

    let server = Server::bind(&addr).serve(make_svc);
    let graceful = server.with_graceful_shutdown(shutdown_signal());
    
    // Run this server for... forever!
    println!("Droplet listening on {} with target {}", addr, dir);
    if let Err(e) = graceful.await {
        eprintln!("server error: {}", e);
    }
}