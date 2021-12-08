use futures::SinkExt;
use futures::StreamExt;

use std::time::Duration;
use url::Url;

use crate::rtc::{
    protocol::{
        client::Message as ClientMessage,
        server::{IncomingMessage, OutgoingMessage, SessionControlMessage},
    },
    server::RtcServer,
    transport::websockets::WebSocketTransport,
};

use serial_test::serial;
use tokio_tungstenite::connect_async;
use tungstenite::Message;

#[tokio::test]
#[serial]
async fn client_handshakes_server() {
    let server = RtcServer::new();
    let (game_tx, game_rx) = server.game_channel();

    let server_runs = server.run(vec![WebSocketTransport::new(([127, 0, 0, 1], 8000).into())]);

    let game_runs = async move {
        info!("Awaiting message in game channel...");
        match game_rx.recv_async().await {
            Ok(message) => match message {
                IncomingMessage::Solicited(_, auth_tx) => {
                    info!("Got handshake, accepting...");
                    auth_tx
                        .send_async(SessionControlMessage::Accept(None))
                        .await
                        .unwrap();
                }
                _ => panic!("Unexpected message!"),
            },
            error @ Err(_) => panic!("{:?}", error),
        }
    };

    let client_runs = async move {
        tokio::time::sleep(Duration::from_millis(1000)).await;

        info!("Connecting...");

        let (client, _) = connect_async(Url::parse("ws://localhost:8000").unwrap())
            .await
            .unwrap();
        info!("Connected!");

        info!("Sending message...");
        let (mut client_tx, mut client_rx) = client.split();
        client_tx
            .send(Message::Binary(
                serde_cbor::ser::to_vec(&ClientMessage::Handshake(None, None)).unwrap(),
            ))
            .await
            .unwrap();

        info!("Waiting for server response...");

        while let Some(message) = client_rx.next().await {
            match message {
                Ok(Message::Binary(bytes)) => match serde_cbor::de::from_slice(bytes.as_slice()) {
                    Ok(ClientMessage::Session(session_id)) => {
                        info!("GOT SESSION ID: {}", session_id);
                        break;
                    }
                    _ => panic!("Deserialization failed"),
                },
                _ => panic!("Incorrect message type"),
            }
        }

        game_tx.send(OutgoingMessage::Halt).unwrap();
    };

    let _result = tokio::join!(server_runs, client_runs, game_runs);
}
