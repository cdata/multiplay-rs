use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use async_std::sync::Mutex;
use async_trait::async_trait;
use flume::{Receiver, Sender};

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

pub fn make_empty_session_state() -> Arc<Mutex<Sessions>> {
    Arc::new(Mutex::new(Sessions::new()))
}

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

    async fn get_outgoing_message_count(&self) -> u32 {
        self.outgoing_message_count.lock().await.clone()
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
