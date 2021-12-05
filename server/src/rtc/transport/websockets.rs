use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};

use crate::rtc::{
    protocol::{
        client::Message,
        common::SessionID,
        server::{IncomingMessage, TransportMessage},
    },
    server::TransportError,
    session::{Sessions, Transport},
};
use async_std::sync::Mutex;
use async_trait::async_trait;
use bimap::BiMap;
use flume::{Receiver, Sender};
use futures::{
    stream::{SplitSink, SplitStream},
    StreamExt,
};
use tokio::select;
use warp::ws::WebSocket;
use warp::{ws::Message as WarpMessage, Filter};

use super::super::server::RtcTransport;
type Connections = BTreeMap<SocketAddr, SplitSink<WebSocket, WarpMessage>>;
type AddressSessions = BiMap<SocketAddr, SessionID>;

struct InternalState {
    connections: Connections,
    address_sessions: AddressSessions,
}

impl InternalState {
    pub fn new() -> Self {
        InternalState {
            connections: Connections::new(),
            address_sessions: AddressSessions::new(),
        }
    }
}

async fn get_session_id(
    shared_state: Arc<Mutex<InternalState>>,
    address: SocketAddr,
) -> Option<SessionID> {
    let state = shared_state.lock().await;
    if let Some(session_id) = state.address_sessions.get_by_left(&address) {
        Some(*session_id)
    } else {
        None
    }
}

#[derive(Clone)]
pub struct WebSocketTransport {
    address: SocketAddr,
    internal_state: Arc<Mutex<InternalState>>,
}

impl WebSocketTransport {
    pub fn new(address: SocketAddr) -> Self {
        WebSocketTransport {
            address,
            internal_state: Arc::new(Mutex::new(InternalState::new())),
        }
    }

    async fn handle_connection(
        internal_state: Arc<Mutex<InternalState>>,
        shared_sessions: Arc<Mutex<Sessions>>,
        connection_address: SocketAddr,
        mut rx: SplitStream<WebSocket>,
        send_to_server: Sender<IncomingMessage>,
    ) {
        while let Some(result) = rx.next().await {
            match result {
                Ok(message) => {
                    if !message.is_binary() {
                        warn!(
                            "Received non-binary Web Socket message from {:?}",
                            connection_address
                        );
                        continue;
                    }

                    match get_session_id(internal_state.clone(), connection_address).await {
                        Some(session_id) => match serde_cbor::de::from_slice(message.as_bytes()) {
                            Ok(Message::Data(message)) => {
                                if let Err(error) = send_to_server
                                    .send_async(IncomingMessage::Received(session_id, message))
                                    .await
                                {
                                    error!("Error forwarding Web Socket message: {:?}", error);
                                }
                            }
                            Err(error) => {
                                error!("Web Socket payload deserialization error: {:?}", error)
                            }
                            _ => warn!("Unexpected Web Socket message payload"),
                        },
                        None => match serde_cbor::de::from_slice(message.as_bytes()) {
                            Ok(Message::Handshake(session_id)) => {
                                match shared_sessions.lock().await.get_mut(&session_id) {
                                    Some(session) => {
                                        if session.transports.contains(&Transport::Bulk) {
                                            warn!("Handshake from session that already has bulk transport!");
                                            continue;
                                        }

                                        let mut state = internal_state.lock().await;
                                        state
                                            .address_sessions
                                            .insert(connection_address, session_id);
                                        session.transports.insert(Transport::Bulk);
                                    }
                                    None => {
                                        error!("No session found for {:?}", session_id);
                                    }
                                }
                            }
                            Err(error) => {
                                error!("Web Socket payload deserialization error: {:?}", error)
                            }
                            _ => {
                                warn!("Unexpected Web Socket message payload")
                            }
                        },
                    }
                }
                Err(error) => {
                    error!("Web Socket receive error: {:?}", error);
                }
            }
        }
    }

    async fn start_warp(
        address: SocketAddr,
        internal_state: Arc<Mutex<InternalState>>,
        shared_sessions: Arc<Mutex<Sessions>>,
        send_to_server: Sender<IncomingMessage>,
    ) -> impl warp::Future {
        let with_internal_state = warp::any().map(move || internal_state.clone());
        let with_shared_sessions = warp::any().map(move || shared_sessions.clone());
        let with_send_to_server = warp::any().map(move || send_to_server.clone());

        let routes = warp::any()
            .and(warp::ws())
            .and(with_internal_state)
            .and(with_shared_sessions)
            .and(with_send_to_server)
            .and(warp::filters::addr::remote())
            .map(
                |ws: warp::ws::Ws,
                 internal_state: Arc<Mutex<InternalState>>,
                 shared_sessions: Arc<Mutex<Sessions>>,
                 send_to_server: Sender<IncomingMessage>,
                 maybe_address| {
                    ws.on_upgrade(move |websocket| async move {
                        match maybe_address {
                            Some(address) => {
                                let (tx, mut rx) = websocket.split();

                                let mut state = internal_state.lock().await;
                                state.connections.insert(address, tx);

                                WebSocketTransport::handle_connection(
                                    internal_state.clone(),
                                    shared_sessions.clone(),
                                    address.clone(),
                                    rx,
                                    send_to_server.clone(),
                                )
                                .await;
                            }
                            _ => {
                                warn!("Skipping connection from unknown address");
                            }
                        };
                    })
                },
            );

        info!("Address: {}", address);

        warp::serve(routes).run(address)
    }

    async fn start_outgoing_message_handler(
        internal_state: Arc<Mutex<InternalState>>,
        shared_sessions: Arc<Mutex<Sessions>>,
        receive_from_server: Receiver<TransportMessage>,
    ) -> impl warp::Future {
        async {}
    }
}

#[async_trait]
impl RtcTransport for WebSocketTransport {
    fn get_type(&self) -> Transport {
        Transport::Bulk
    }

    async fn start(
        self,
        shared_sessions: Arc<Mutex<Sessions>>,
        send_to_server: Sender<IncomingMessage>,
        receive_from_server: Receiver<TransportMessage>,
    ) -> Result<(), TransportError> {
        let warp_runs = WebSocketTransport::start_warp(
            self.address.clone(),
            self.internal_state.clone(),
            shared_sessions.clone(),
            send_to_server,
        );

        let outgoing_message_handler_runs = WebSocketTransport::start_outgoing_message_handler(
            self.internal_state.clone(),
            shared_sessions.clone(),
            receive_from_server,
        );

        select! {
            _ = warp_runs => warn!("WebSocket server has stopped"),
            _ = outgoing_message_handler_runs => warn!("Outgoing message handler has stopped")
        }

        Ok(())
    }

    // async fn send(self, session_ids: &Vec<SessionID>, data: &[u8]) -> Result<(), TransportError> {
    //     let internal_state = self.internal_state.clone();

    //     for session_id in session_ids {
    //         // TODO: Handle TransportError conditions...
    //         let result = match internal_state
    //             .lock()
    //             .await
    //             .address_sessions
    //             .get_by_right(session_id)
    //         {
    //             Some(address) => match internal_state.lock().await.connections.get_mut(&address) {
    //                 Some(tx) => match tx.send(WarpMessage::binary(data)).await {
    //                     Err(error) => {
    //                         error!("Web Socket send error: {:?}", error);
    //                         Err(TransportError::SendFailed)
    //                     }
    //                     _ => Ok(()),
    //                 },
    //                 None => {
    //                     warn!("No Web Socket connection for session {:?}", session_id);
    //                     Err(TransportError::NoConnection)
    //                 }
    //             },
    //             None => {
    //                 warn!("No Web Socket address for session {:?}", session_id);
    //                 Err(TransportError::NoAddress)
    //             }
    //         };

    //         match result {
    //             Err(error) => {
    //                 error!("{:?}", error);
    //             }
    //             _ => (),
    //         }
    //     }

    //     Ok(())
    // }

    // async fn purge(&mut self, session_id: SessionID) -> Result<(), TransportError> {
    //     let shared_state = self.internal_state.clone();
    //     let state = shared_state.lock().await;
    //     let address = state.address_sessions.get_by_right(&session_id);

    //     match address {
    //         Some(address) => {
    //             let mut state = self.internal_state.lock().await;
    //             state.connections.remove(address);
    //             state.address_sessions.remove_by_left(address);
    //         }
    //         _ => (),
    //     };

    //     Ok(())
    // }
}
