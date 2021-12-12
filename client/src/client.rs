use std::{sync::Arc, thread::JoinHandle};

use anyhow::{anyhow, Context, Result};
use async_std::sync::Mutex;
use flume::{Receiver, Sender};
use futures::{SinkExt, StreamExt};
use multiplay_rs::rtc::protocol::{client::Message, common::SessionID};
use serde::{de::DeserializeOwned, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::Message as TungsteniteMessage, MaybeTlsStream, WebSocketStream,
};
use url::Url;

pub struct Client {
    join_handle: Option<JoinHandle<Result<()>>>,
    stop_thread_tx: Option<Sender<()>>,
    session_id: Arc<Mutex<Option<SessionID>>>,
}

impl Client {
    pub fn new() -> Self {
        Client {
            join_handle: None,
            stop_thread_tx: None,
            session_id: Arc::new(Mutex::new(None)),
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
        let session_id = self.session_id.clone();

        let join_handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let message_loop_result = rt.block_on(client_message_loop(
                url,
                session_id,
                to_consumer_tx,
                from_consumer_rx,
                stop_thread_rx,
            ));

            if let Err(error) = &message_loop_result {
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
    session_id: Arc<Mutex<Option<SessionID>>>,
    message_tx: Sender<T>,
    message_rx: Receiver<T>,
    stop_rx: Receiver<()>,
) -> Result<()>
where
    T: Send + Serialize + DeserializeOwned,
{
    let (ws_tx, mut ws_rx) = web_socket_connect(url).await?.split();
    let (auth_tx, auth_rx) = flume::bounded::<SessionID>(1);
    let ws_tx = Arc::new(Mutex::new(ws_tx));

    let send_loop = async {
        // Handshake to server, passing a ready-available session ID if
        // we have one:
        // TODO: Pass admin secret if we have one...
        match ws_tx
            .lock()
            .await
            .send(web_socket_payload(Message::Handshake(
                *session_id.lock().await,
                None,
            )))
            .await
        {
            Err(error) => {
                error!("Failed to send handshake to server: {:?}", error);
                return ();
            }
            _ => (),
        };

        match auth_rx.recv_async().await {
            Ok(_session_id) => {
                while let Ok(message) = message_rx.recv_async().await {
                    match serde_cbor::ser::to_vec(&message) {
                        Ok(data) => match ws_tx
                            .lock()
                            .await
                            .send(web_socket_payload(Message::Data(data)))
                            .await
                        {
                            Err(error) => error!("Failed to send message: {:?}", error),
                            _ => (),
                        },
                        Err(error) => error!("Failed to serialize data: {:?}", error),
                    };
                }
            }
            Err(error) => error!("Failed to resolve session ID: {:?}", error),
        }
    };

    let receive_loop = async {
        while let Some(Ok(message)) = ws_rx.next().await {
            match message {
                TungsteniteMessage::Binary(data) => {
                    match serde_cbor::de::from_slice(data.as_slice()) {
                        Ok(Message::Session(session_id)) => {
                            match auth_tx.send_async(session_id).await {
                                Err(error) => {
                                    error!("Failed to forward auth to send loop: {:?}", error)
                                }
                                _ => (),
                            }
                        }
                        Ok(Message::Data(data)) => match serde_cbor::de::from_slice::<T>(&data) {
                            Ok(message) => match message_tx.send_async(message).await {
                                Err(error) => {
                                    error!("Failed send message to embedder: {:?}", error)
                                }
                                _ => (),
                            },
                            Err(error) => error!("Failed to deserialize message: {:?}", error),
                        },
                        Ok(Message::Ping(ping_id)) => {
                            if let Err(error) = ws_tx
                                .lock()
                                .await
                                .send(web_socket_payload(Message::Pong(ping_id)))
                                .await
                            {
                                error!("Failed to send ping response to server: {:?}", error);
                            }
                        }
                        _ => warn!("Unanticipated result deserializing server message"),
                    }
                }
                _ => warn!("Unexpected message type received from server"),
            }
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
