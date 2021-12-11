use std::{sync::Arc, thread::JoinHandle};

use anyhow::{anyhow, Context, Result};
use async_std::sync::Mutex;
use flume::{Receiver, Sender};
use futures::{SinkExt, StreamExt};
use multiplay_rs::rtc::protocol::{client::Message, common::SessionID};
use serde::{de::DeserializeOwned, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use url::Url;

struct Client {
    join_handle: Option<JoinHandle<Result<()>>>,
    stop_thread_tx: Option<Sender<()>>,
}

impl Client {
    pub fn new() -> Self {
        Client {
            join_handle: None,
            stop_thread_tx: None,
        }
    }

    pub fn is_open(&self) -> bool {
        match (self.join_handle.as_ref(), self.stop_thread_tx.as_ref()) {
            (None, None) => false,
            _ => true,
        }
    }

    pub fn connect<T>(&mut self, url: &'static str) -> (Sender<T>, Receiver<T>)
    where
        T: Send + Serialize + DeserializeOwned + 'static,
    {
        let (stop_thread_tx, stop_thread_rx) = flume::unbounded::<()>();
        let (to_client_tx, from_consumer_rx) = flume::unbounded::<T>();
        let (to_consumer_tx, from_client_rx) = flume::unbounded::<T>();

        let join_handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let message_loop_result = rt.block_on(client_message_loop(
                to_consumer_tx,
                from_consumer_rx,
                stop_thread_rx,
            ));

            if let Err(error) = message_loop_result {
                error!("{:?}", error);
            }

            message_loop_result
        });

        self.join_handle = Some(join_handle);
        self.stop_thread_tx = Some(stop_thread_tx);

        (to_client_tx, from_client_rx)
    }

    pub async fn close(mut self) -> Result<()> {
        if let Some(tx) = self.stop_thread_tx {
            let result = tx.send_async(()).await;

            if let Err(_) = result {
                return result.context("Failed to send stop message to client thread.");
            }
        }

        if let Some(handle) = self.join_handle {
            let result = handle.join();

            if let Err(_) = result {
                // TODO: How to forward error here?
                return Err(anyhow!("Failed to join client thread."));
            }
        }

        self.join_handle = None;
        self.stop_thread_tx = None;

        Ok(())
    }
}

async fn client_message_loop<T>(
    url: &'static str,
    message_tx: Sender<T>,
    message_rx: Receiver<T>,
    stop_rx: Receiver<()>,
) -> Result<()>
where
    T: Send + Serialize + DeserializeOwned,
{
    let (mut ws_tx, mut ws_rx) = web_socket_connect(url).await?.split();
    let session_id: Arc<Mutex<Option<SessionID>>> = Arc::new(Mutex::new(None));

    let send_loop = async {
        while let Ok(message) = message_rx.recv_async().await {
            if let None = *session_id.lock().await {
                continue;
            }

            match serde_cbor::ser::to_vec(&message) {
                Ok(data) => match ws_tx.send(web_socket_payload(Message::Data(data))).await {
                    Err(error) => error!("Failed to send message: {:?}", error),
                    _ => (),
                },
                Err(error) => error!("Failed to serialize data: {:?}", error),
            };
        }
    };

    let receive_loop = async {
        while let Some(message) = ws_rx.next().await {
            // match message {
            //     Ok(WsMessage::Binary(bytes)) => {
            //         let message: Message =
            //             serde_cbor::de::from_slice(bytes.as_slice()).unwrap();
            //         send_to_server.send_async(message).await.unwrap();
            //     }
            //     _ => panic!("Incorrect message type"),
            // }
        }
    };

    let kill_loop = async {
        stop_rx.recv_async().await.unwrap();
    };

    tokio::select! {
        _ = send_loop => (),
        _ = receive_loop => (),
        _ = kill_loop => ()
    };

    Ok(())
}

async fn web_socket_connect(url: &str) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    match connect_async(Url::parse(url).unwrap()).await {
        Ok((client, _)) => Ok(client),
        Err(error) => {
            Err(error).context(format!("Unable to connect to {} over a web socket!", url))
        }
    }
}

fn web_socket_payload(message: Message) -> tokio_tungstenite::tungstenite::Message {
    let bytes = serde_cbor::ser::to_vec(&message).unwrap();
    tokio_tungstenite::tungstenite::Message::Binary(bytes)
}

/*


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

*/
