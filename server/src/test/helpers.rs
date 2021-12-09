use anyhow::Result;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::Message as WsMessage, MaybeTlsStream, WebSocketStream,
};
use url::Url;

use async_std::sync::Mutex;
use async_trait::async_trait;
use flume::{Receiver, Sender};

use crate::rtc::protocol::client::Message;
use crate::rtc::protocol::server::{TransportIncomingMessage, TransportOutgoingMessage};
use crate::rtc::server::RtcTransport;
use crate::rtc::session::{Sessions, Transport};

pub fn init_logging() {
    // NOTE(cdata): pretty_env_logger's builder seems busted
    // See: https://github.com/seanmonstar/pretty-env-logger/issues/43
    pretty_env_logger::env_logger::Builder::from_default_env()
        .is_test(true)
        .init();
}

// pub fn make_empty_session_state() -> Arc<Mutex<Sessions>> {
//     Arc::new(Mutex::new(Sessions::new()))
// }

#[derive(Serialize, Deserialize)]
pub enum TestProtocol {
    Stop,
}

pub struct TestTransport {
    outgoing_message_count: Arc<Mutex<u32>>,
}

impl TestTransport {
    pub fn with_state(outgoing_message_count: Arc<Mutex<u32>>) -> Self {
        TestTransport {
            outgoing_message_count,
        }
    }
}

#[async_trait]
impl RtcTransport for TestTransport {
    fn get_type(&self) -> Transport {
        Transport::Bulk
    }

    async fn start(
        self,
        _shared_sessions: Arc<Mutex<Sessions>>,
        _tx: Sender<TransportIncomingMessage>,
        rx: Receiver<TransportOutgoingMessage>,
    ) -> Result<()> {
        info!("Starting test transport...");
        while let Ok(message) = rx.recv_async().await {
            *self.outgoing_message_count.lock().await += 1;
            info!("Got message...");
            match message {
                TransportOutgoingMessage::Send(_, data) => {
                    match serde_cbor::de::from_slice(data.as_slice()) {
                        Ok(TestProtocol::Stop) => {
                            info!("Stopping!");
                            break;
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }
}

pub async fn ws_connect(url: &str) -> WebSocketStream<MaybeTlsStream<TcpStream>> {
    match connect_async(Url::parse(url).unwrap()).await {
        Ok((client, _)) => client,
        _ => panic!("Unable to connect to {} over a web socket!", url),
    }
}

pub fn ws_payload(message: Message) -> WsMessage {
    let bytes = serde_cbor::ser::to_vec(&message).unwrap();
    WsMessage::Binary(bytes)
}

pub struct WsTestClient {
    join_handle: JoinHandle<()>,
    stop_thread_tx: Sender<()>,
    pub rx: Receiver<Message>,
    pub tx: Sender<Message>,
}

impl WsTestClient {
    pub fn new(url: &'static str) -> Self {
        let (stop_thread_tx, stop_thread_rx) = flume::unbounded::<()>();
        let (send_to_server, receive_from_client) = flume::unbounded::<Message>();
        let (send_to_client, receive_from_server) = flume::unbounded::<Message>();

        let join_handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                info!("Test client connecting...");
                let (mut ws_tx, mut ws_rx) = ws_connect(url).await.split();

                let send_loop = async {
                    while let Ok(message) = receive_from_server.recv_async().await {
                        ws_tx.send(ws_payload(message)).await.unwrap();
                    }
                };

                let receive_loop = async {
                    while let Some(message) = ws_rx.next().await {
                        match message {
                            Ok(WsMessage::Binary(bytes)) => {
                                let message: Message =
                                    serde_cbor::de::from_slice(bytes.as_slice()).unwrap();
                                send_to_server.send_async(message).await.unwrap();
                            }
                            _ => panic!("Incorrect message type"),
                        }
                    }
                };

                let kill_loop = async {
                    stop_thread_rx.recv_async().await.unwrap();
                };

                tokio::select! {
                    _ = send_loop => (),
                    _ = receive_loop => (),
                    _ = kill_loop => ()
                };
            });
        });

        WsTestClient {
            join_handle,
            stop_thread_tx,
            tx: send_to_client,
            rx: receive_from_client,
        }
    }

    pub fn close(self) {
        self.stop_thread_tx.send(()).unwrap();
        self.join_handle.join().unwrap();
    }
}
