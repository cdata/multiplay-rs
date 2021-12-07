extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use std::path::Path;

use multiplay_rs::{
    assets::serve_assets,
    rtc::{server::RtcServer, transport::websockets::WebSocketTransport},
};

use tokio::select;

#[tokio::main]
pub async fn main() {
    pretty_env_logger::init();

    info!("Starting server example...");

    let static_server_runs = serve_assets(Path::new("demo"), ([127, 0, 0, 1], 8000).into());

    let rtc_server = RtcServer::new();

    let rtc_server_runs =
        rtc_server.run(vec![WebSocketTransport::new(([127, 0, 0, 1], 8001).into())]);

    select! {
        _ = static_server_runs => info!("Static server stopped"),
        _ = rtc_server_runs => info!("RTC server stopped")
    }

    info!("Server example over!");
}
