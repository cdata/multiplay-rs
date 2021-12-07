mod helpers;
use std::sync::Arc;

use crate::rtc::protocol::server::TransportIncomingMessage;
use crate::rtc::protocol::server::TransportOutgoingMessage;
use crate::rtc::server::RtcServer;
use crate::rtc::server::RtcTransport;
use crate::rtc::session::Sessions;
use crate::rtc::session::Transport;
use anyhow::Result;
use async_std::sync::Mutex;
use async_trait::async_trait;
use flume::Receiver;
use flume::Sender;

struct TestTransport {
    state: u32,
}

#[async_trait]
impl RtcTransport for TestTransport {
    fn get_type(&self) -> Transport {
        Transport::Bulk
    }

    async fn start(
        self,
        shared_sessions: Arc<Mutex<Sessions>>,
        tx: Sender<TransportIncomingMessage>,
        rx: Receiver<TransportOutgoingMessage>,
    ) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn start_server() {
    helpers::init_logging();

    let rtc_server = RtcServer::new();

    let result = rtc_server.run(vec![TestTransport { state: 0 }]).await;

    println!("Result: {:?}", result);
}

/*
mod sessions {
    use super::helpers;
    use crate::rtc::{
        protocol::server::{
            IncomingMessage, OutgoingMessage, SessionControlMessage, SessionPayload,
        },
        _sessions::serve_sessions,
    };
    use ntest::timeout;

    use hyper::{Body, Client as HttpClient, Method, Request};
    use tokio::join;

    #[tokio::test]
    #[timeout(1000)]
    async fn are_created_pending_authentication() {
        helpers::init_logging();

        let (send_to_game, receive_from_server) = flume::unbounded::<IncomingMessage>();
        let (_send_to_server, receive_from_game) = flume::unbounded::<OutgoingMessage>();
        let shared_sessions = helpers::make_empty_session_state();

        debug!("Starting RTC session server...");

        let server_thread = serve_sessions(
            shared_sessions,
            ([127, 0, 0, 1], 8001).into(),
            send_to_game,
            receive_from_game,
        );

        let client_thread = async move {
            let client = HttpClient::new();

            debug!("Making request...");

            let response = client
                .request(
                    Request::builder()
                        .method(Method::POST)
                        .uri("http://localhost:8001/session")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            debug!("Got response!");

            let bytes = hyper::body::to_bytes(response).await.unwrap();

            debug!("Bytes: {:?}", bytes);

            let payload: SessionPayload = serde_json::from_slice(&bytes).unwrap();

            debug!("Payload: {:?}", payload);

            server_thread.abort();
        };

        let game_thread = async move {
            debug!("Starting fake game thread...");
            let (id, auth_sender) = match receive_from_server.recv_async().await {
                Ok(IncomingMessage::Solicited(id, _socket_addr, auth_sender)) => (id, auth_sender),
                Ok(_) => panic!("Unexpected message!"),
                Err(error) => panic!("Control message receive error: {:?}", error),
            };

            debug!("Accepting authentication request");

            auth_sender
                .send_async(SessionControlMessage::Accept(id))
                .await
        };

        let _ = join! {
          client_thread,
          game_thread
        };
    }
}
*/
