use std::{net::SocketAddr, path::Path};

use tokio::task;
use warp;

async fn serve_assets_task(path: &'static Path, address: SocketAddr) {
    info!("Starting assets server...");
    warp::serve(warp::fs::dir(path)).run(address).await;
}

pub async fn serve_assets(path: &'static Path, address: SocketAddr) {
    let result = task::spawn(serve_assets_task(path, address)).await;
    if let Err(error) = result {
        error!("Assets server error: {:?}", error);
    }
}
