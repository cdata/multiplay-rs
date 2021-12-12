use std::{net::SocketAddr, path::Path};

use tokio::task;
use warp;

pub async fn serve_assets(path: &'static Path, address: SocketAddr) {
    info!("Serving static assets on {:?}...", address);
    let result = task::spawn(warp::serve(warp::fs::dir(path)).run(address)).await;
    if let Err(error) = result {
        error!("Assets server error: {:?}", error);
    }
}
