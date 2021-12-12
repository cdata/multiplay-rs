use crate::rtc::{
    protocol::{
        client::Message,
        server::{IncomingMessage, OutgoingMessage, SessionControlMessage},
    },
    server::RtcServer,
    transport::websockets::WebSocketTransport,
};

use serial_test::serial;

use super::helpers::WsTestClient;

#[tokio::test]
#[serial]
async fn client_handshakes_server() {
    let server = RtcServer::new();
    let (game_tx, game_rx) = server.game_channel();

    let server_runs = server.run(vec![WebSocketTransport::new(([127, 0, 0, 1], 8000).into())]);

    let game_runs = async move {
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
        let client = WsTestClient::new("ws://localhost:8000");

        client
            .tx
            .send_async(Message::Handshake(None, None))
            .await
            .unwrap();

        info!("Waiting for server response...");

        while let Ok(message) = client.rx.recv_async().await {
            match message {
                Message::Session(session_id) => {
                    info!("Got session ID: {}", session_id);
                    break;
                }
                _ => panic!("Deserialization failed"),
            }
        }

        client.close();
        game_tx.send(OutgoingMessage::Halt).unwrap();
    };

    let _result = tokio::join!(server_runs, client_runs, game_runs);
}

mod admin {
    use serial_test::serial;

    use crate::rtc::{
        protocol::{client::Message, server::OutgoingMessage},
        server::RtcServer,
        transport::websockets::WebSocketTransport,
    };

    use super::WsTestClient;

    #[tokio::test]
    #[serial]
    async fn client_handshakes_as_admin() {
        super::super::helpers::init_logging();

        let server = RtcServer::new();
        let admin_secret = server.get_admin_secret();
        let (game_tx, _game_rx) = server.game_channel();

        let server_runs = server.run(vec![WebSocketTransport::new(([127, 0, 0, 1], 8000).into())]);

        let client_runs = async {
            let client = WsTestClient::new("ws://localhost:8000");

            client
                .tx
                .send_async(Message::Handshake(None, Some(admin_secret)))
                .await
                .unwrap();

            info!("Waiting for server response...");

            while let Ok(message) = client.rx.recv_async().await {
                match message {
                    Message::Session(session_id) => {
                        info!("Got session ID: {}", session_id);
                        break;
                    }
                    _ => panic!("Deserialization failed"),
                }
            }

            client.close();

            game_tx.send_async(OutgoingMessage::Halt).await.unwrap();
        };

        let _result = tokio::join!(server_runs, client_runs);
    }
}
