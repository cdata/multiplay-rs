extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use std::{path::Path, sync::Arc};

use async_std::sync::Mutex;
use multiplay_rs::assets::serve_assets;
use multiplay_rs::rtc::protocol::{IncomingMessage, OutgoingMessage};
use multiplay_rs::rtc::serve::serve_rtc_sessions;
use multiplay_rs::rtc::session::Sessions;

#[tokio::main]
pub async fn main() {
    pretty_env_logger::init();

    info!("Starting server example...");

    let (send_to_game, _receive_from_server) = flume::unbounded::<IncomingMessage<u32>>();
    let (_send_to_server, receive_from_game) = flume::unbounded::<OutgoingMessage<u32>>();

    let static_server = serve_assets(Path::new("demo"), ([127, 0, 0, 1], 8000).into());
    let rtc_session_manager = serve_rtc_sessions(
        Arc::new(Mutex::new(Sessions::new())),
        ([127, 0, 0, 1], 8001).into(),
        send_to_game.clone(),
        receive_from_game.clone(),
    );

    info!("Static server started...");

    let (_, result) = futures::join!(static_server, rtc_session_manager);

    result.unwrap();

    info!("Server example complete!");
}
