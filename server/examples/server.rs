extern crate pretty_env_logger;
#[macro_use]
extern crate log;

// use std::path::Path;

// use multiplay_rs::{
//     assets::serve_assets,
//     rtc::{server::RtcServer, transport::websockets::WebSocketTransport},
// };

#[tokio::main]
pub async fn main() {
    pretty_env_logger::init();

    info!("Starting server example...");

    // let static_server_runs = serve_assets(Path::new("demo"), ([127, 0, 0, 1], 8000).into());

    // let rtc_server = RtcServer::new();

    // let rtc_server_runs = rtc_server.run(
    //     ([127, 0, 0, 1], 8001).into(),
    //     vec![WebSocketTransport::new(([127, 0, 0, 1], 8002).into())],
    // );

    // tokio::select! {
    //     _ = static_server_runs => println!("Static server stopped"),
    //     _ = rtc_server_runs => println!("RTC server stopped")
    // }

    info!("Server example over!");
    // let rtc_server = RtcServer::new(vec![WebSocketTransport::new(([127, 0, 0, 1], 8002).into())]);
    // let rtc_server_runs = rtc_server.run(([127, 0, 0, 1], 8001).into());

    // let (send_to_game, receive_from_server) = flume::unbounded::<IncomingMessage>();
    // let (send_to_server, receive_from_game) = flume::unbounded::<OutgoingMessage>();

    // let shared_sessions = Arc::new(Mutex::new(Sessions::new()));

    // let static_server = serve_assets(Path::new("demo"), ([127, 0, 0, 1], 8000).into());
    // let session_server = serve_sessions(
    //     shared_sessions.clone(),
    //     ([127, 0, 0, 1], 8001).into(),
    //     send_to_game.clone(),
    //     receive_from_game.clone(),
    // );

    // let websocket_server = serve_websockets(
    //     shared_sessions.clone(),
    //     ([127, 0, 0, 1], 8002).into(),
    //     send_to_game.clone(),
    //     receive_from_game.clone(),
    // );

    // info!("Static server started...");

    // let (_, result1, result2) = futures::join!(static_server, session_server, websocket_server);

    // result1.unwrap();
    // result2.unwrap();

    // info!("Server example complete!");
}
