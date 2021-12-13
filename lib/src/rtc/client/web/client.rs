use std::sync::Arc;

use crate::rtc::protocol::client::Message;
use crate::rtc::protocol::common::SessionID;

use super::panic::set_panic_hook;
use async_std::sync::Mutex;
use flume::{Receiver, Sender};
use js_sys::{ArrayBuffer, Uint8Array};
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{BinaryType, MessageEvent, WebSocket};

pub struct Client {
    session_id: Arc<Mutex<Option<SessionID>>>,
    admin_secret: Arc<Mutex<Option<String>>>,
}

impl Client {
    pub fn new(admin_secret: Option<String>) -> Self {
        set_panic_hook();
        Client {
            session_id: Arc::new(Mutex::new(None)),
            admin_secret: Arc::new(Mutex::new(admin_secret)),
        }
    }

    pub fn is_open(&self) -> bool {
        true
    }

    pub fn tick() {}

    pub fn connect<T>(&mut self, url: String) -> (Sender<T>, Receiver<T>)
    where
        T: Serialize + DeserializeOwned + 'static,
    {
        let (to_client_tx, from_consumer_rx) = flume::unbounded::<T>();
        let (to_consumer_tx, from_client_rx) = flume::unbounded::<T>();

        let web_socket = WebSocket::new(&url).unwrap();

        web_socket.set_binary_type(BinaryType::Arraybuffer);

        let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
            match event.data().dyn_into::<ArrayBuffer>() {
                Ok(buffer) => {
                    let array = Uint8Array::new(&buffer);
                    match serde_cbor::de::from_slice::<Message>(array.to_vec().as_slice()) {
                        Ok(message) => match message {
                            Message::Session(_) => todo!(),
                            Message::Data(_) => todo!(),
                            Message::Ping(_) => todo!(),
                            _ => warn!("Unexpected message from server"),
                        },
                        Err(error) => {
                            error!("Failed to deserialize message from server: {:?}", error)
                        }
                    }
                }
                _ => (),
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        let session_id = self.session_id.clone();
        let on_open_web_socket = web_socket.clone();
        let on_open = Closure::wrap(Box::new(move |_| {
            info!("Web Socket opened!");
            async_std::task::block_on(async {
                let session_id = session_id.lock().await;
                match serde_cbor::ser::to_vec(&Message::Handshake(*session_id, None)) {
                    Ok(handshake_payload) => {
                        match on_open_web_socket.send_with_u8_array(handshake_payload.as_slice()) {
                            Err(error) => error!("Failed to send handshake to server: {:?}", error),
                            _ => (),
                        }
                    }
                    Err(error) => error!("Failed to serialize handshake: {:?}", error),
                }

                while let Ok(message) = from_consumer_rx.recv_async().await {
                    match serde_cbor::ser::to_vec(&message) {
                        Ok(data) => match serde_cbor::ser::to_vec(&Message::Data(data)) {
                            Ok(payload) => {
                                match on_open_web_socket.send_with_u8_array(payload.as_slice()) {
                                    Err(error) => {
                                        error!("Failed to send data to server: {:?}", error)
                                    }
                                    _ => (),
                                }
                            }
                            Err(error) => {
                                error!("Failed to serialize message payload: {:?}", error)
                            }
                        },
                        Err(error) => error!("Failed to serialize message data: {:?}", error),
                    }
                }
            });
        }) as Box<dyn FnMut(JsValue)>);

        web_socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        web_socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));

        (to_client_tx, from_client_rx)
    }
}

struct WebSocketTransport<T>
where
    T: Serialize + DeserializeOwned + 'static,
{
    web_socket: Option<WebSocket>,
    tx: Sender<T>,
    rx: Receiver<T>,
}

impl<T> WebSocketTransport<T>
where
    T: Serialize + DeserializeOwned + 'static,
{
    pub fn new(tx: Sender<Message>, rx: Receiver<Message>) -> Self {
        let web_socket = WebSocket::new(&url).unwrap();
    }

    fn handle_message(&mut self, message: Message) {
        match message {
            Message::Session(_) => todo!(),
            Message::Data(_) => todo!(),
            Message::Ping(_) => todo!(),
            _ => warn!("Unexpected message from server"),
        };
    }
}
