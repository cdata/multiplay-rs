use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};

use crate::rtc::{
    protocol::{
        client::Message,
        common::SessionID,
        server::{SessionControlMessage, TransportIncomingMessage, TransportOutgoingMessage},
    },
    server::RtcTransport,
    session::Transport,
};

use anyhow::{anyhow, Context, Result};
use async_std::sync::Mutex;
use async_trait::async_trait;
use bimap::BiMap;
use flume::{Receiver, Sender};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::select;
use warp::{
    ws::{Message as WarpMessage, WebSocket},
    Filter,
};

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

    pub fn get_connection(
        &mut self,
        id: &SessionID,
    ) -> Option<&mut SplitSink<WebSocket, WarpMessage>> {
        let address = self.address_sessions.get_by_right(id)?;
        self.connections.get_mut(address)
    }
}

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
}

#[async_trait]
impl RtcTransport for WebSocketTransport {
    fn get_type(&self) -> Transport {
        Transport::Bulk
    }

    async fn start(
        self,
        send_to_server: Sender<TransportIncomingMessage>,
        receive_from_server: Receiver<TransportOutgoingMessage>,
    ) -> Result<()> {
        let warp_runs = start_warp(
            self.address.clone(),
            self.internal_state.clone(),
            send_to_server,
        );

        let outgoing_message_handler_runs =
            start_outgoing_message_handler(receive_from_server, self.internal_state.clone());

        select! {
            _ = warp_runs => info!("WebSocket server has stopped"),
            _ = outgoing_message_handler_runs => info!("Outgoing message handler has stopped")
        }

        Ok(())
    }
}

async fn start_warp(
    address: SocketAddr,
    internal_state: Arc<Mutex<InternalState>>,
    send_to_server: Sender<TransportIncomingMessage>,
) -> Result<()> {
    let with_internal_state = warp::any().map(move || internal_state.clone());
    let with_send_to_server = warp::any().map(move || send_to_server.clone());

    let routes = warp::any()
        .and(warp::ws())
        .and(with_internal_state)
        .and(with_send_to_server)
        .and(warp::filters::addr::remote())
        .map(
            |ws: warp::ws::Ws,
             internal_state: Arc<Mutex<InternalState>>,
             send_to_server: Sender<TransportIncomingMessage>,
             maybe_address| {
                info!("Warp connection started......");

                ws.on_upgrade(move |websocket| async move {
                    match maybe_address {
                        Some(address) => {
                            let (tx, rx) = websocket.split();

                            let mut state = internal_state.lock().await;
                            state.connections.insert(address, tx);
                            drop(state);

                            handle_connection(
                                rx,
                                internal_state.clone(),
                                address.clone(),
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

    info!("Starting warp on address: {}", address);

    warp::serve(routes).run(address).await;

    Ok(())
}

/**
 * Runs the message receive loop for client connections
 */
async fn handle_connection(
    mut rx: SplitStream<WebSocket>,
    internal_state: Arc<Mutex<InternalState>>,
    connection_address: SocketAddr,
    send_to_server: Sender<TransportIncomingMessage>,
) {
    while let Some(message_result) = rx.next().await {
        let result: Result<()> = match message_result {
            Ok(message) => {
                if message.is_close() {
                    Err(anyhow!("Connection closed by client"))
                } else if !message.is_binary() {
                    warn!(
                        "Received non-binary Web Socket message from {:?}",
                        connection_address
                    );
                    continue;
                } else {
                    handle_message(
                        message,
                        internal_state.clone(),
                        connection_address.clone(),
                        send_to_server.clone(),
                    )
                    .await
                }
            }
            Err(error) => Err(error).context("Web Socket receive error"),
        };

        match result {
            Err(error) => {
                error!("{:?}", error);
                close_connection(
                    internal_state.clone(),
                    connection_address.clone(),
                    send_to_server.clone(),
                )
                .await;
                break;
            }
            _ => (),
        };
    }
}

/**
 * Processes a single message from a single client
 */
async fn handle_message(
    message: WarpMessage,
    internal_state: Arc<Mutex<InternalState>>,
    connection_address: SocketAddr,
    send_to_server: Sender<TransportIncomingMessage>,
) -> Result<()> {
    debug!(
        "Received message from {:?}: {:?}",
        connection_address, message
    );
    match get_session_id(internal_state.clone(), connection_address).await {
        Some(session_id) => match serde_cbor::de::from_slice(message.as_bytes()) {
            Ok(Message::Data(message)) => match send_to_server
                .send_async(TransportIncomingMessage::Received(session_id, message))
                .await
            {
                error @ Err(_) => error.context("Error forwarding message data to server"),
                _ => Ok(()),
            },
            Err(error) => Err(error).context("Web Socket payload deserialization error"),
            _ => {
                warn!("Unexpected Web Socket message payload");
                Ok(())
            }
        },
        None => {
            match serde_cbor::de::from_slice(message.as_bytes()) {
                Ok(Message::Handshake(maybe_session_id, maybe_admin_secret)) => {
                    match maybe_session_id {
                        Some(session_id) => {
                            let mut state = internal_state.lock().await;
                            state
                                .address_sessions
                                .insert(connection_address, session_id);

                            match send_to_server
                                .send_async(TransportIncomingMessage::Connected(
                                    session_id,
                                    maybe_admin_secret,
                                ))
                                .await
                            {
                                Err(error) => {
                                    Err(error).context("Error forwarding handshake to server")
                                }
                                _ => Ok(()),
                            }
                        }
                        None => {
                            // Connecting for the first time...
                            let (auth_tx, auth_rx) = flume::unbounded::<SessionControlMessage>();

                            if let Err(error) = send_to_server
                                .send_async(TransportIncomingMessage::Solicited(
                                    Some(connection_address.clone()),
                                    maybe_admin_secret,
                                    auth_tx,
                                ))
                                .await
                            {
                                Err(error)
                                    .context("Failed to forward new session handshake to server")
                            } else {
                                info!("Awaiting auth.....");
                                match auth_rx.recv_async().await {
                                    Ok(SessionControlMessage::Accept(Some(session_id))) => {
                                        info!("Got session ID for authing client: {}", session_id);
                                        let mut state = internal_state.lock().await;
                                        state
                                            .address_sessions
                                            .insert(connection_address, session_id);

                                        match state.connections.get_mut(&connection_address) {
                                            Some(tx) => match serde_cbor::ser::to_vec(
                                                &Message::Session(session_id),
                                            ) {
                                                Ok(payload) => tx
                                                    .send(WarpMessage::binary(payload))
                                                    .await
                                                    .context("Failed to send message to client"),
                                                _ => Err(anyhow!(
                                                    "Failed to serialize message for client"
                                                )),
                                            },
                                            None => Err(anyhow!(
                                                "No sink half of the client WebSocket stream!"
                                            )),
                                        }
                                    }
                                    Ok(SessionControlMessage::Reject(reason)) => {
                                        Err(anyhow!("Session rejected (reason: {:?})", reason))
                                    }
                                    Ok(SessionControlMessage::Accept(None)) => Err(anyhow!(
                                        "Connection was accepted without a session ID!"
                                    )),
                                    Err(error) => Err(error)
                                        .context("Error waiting for session authentication"),
                                }
                            }
                        }
                    }
                }
                Err(error) => Err(error).context("Web Socket payload deserialization error"),
                _ => {
                    warn!("Unexpected Web Socket message payload");
                    Ok(())
                }
            }
        }
    }
}

/**
 * Look up the session ID for a socket address in internal state
 */
async fn get_session_id(
    internal_state: Arc<Mutex<InternalState>>,
    address: SocketAddr,
) -> Option<SessionID> {
    let state = internal_state.lock().await;
    if let Some(session_id) = state.address_sessions.get_by_left(&address) {
        Some(*session_id)
    } else {
        None
    }
}

/**
 * Close a connection, purging it from internal state
 */
async fn close_connection(
    internal_state: Arc<Mutex<InternalState>>,
    connection_address: SocketAddr,
    send_to_server: Sender<TransportIncomingMessage>,
) {
    info!("Closing connection ({:?})", connection_address);
    let session_id = get_session_id(internal_state.clone(), connection_address.clone()).await;
    let mut state = internal_state.lock().await;

    state.connections.remove(&connection_address);
    state.address_sessions.remove_by_left(&connection_address);

    if let Some(session_id) = session_id {
        if let Err(error) = send_to_server
            .send_async(TransportIncomingMessage::Disconnected(session_id))
            .await
        {
            error!("Error notifying server of closed connection: {:?}", error);
        }
    }
}

/**
 * Handle outgoing messages from the server
 */
async fn start_outgoing_message_handler(
    rx: Receiver<TransportOutgoingMessage>,
    internal_state: Arc<Mutex<InternalState>>,
) -> Result<()> {
    // TODO
    while let Ok(message) = rx.recv_async().await {
        match message {
            TransportOutgoingMessage::Send(session_ids, data) => {
                match serde_cbor::ser::to_vec(&Message::Data(data)) {
                    Ok(payload) => {
                        let warp_message = WarpMessage::binary(payload);

                        let send_futures = session_ids.iter().map(|session_id| async {
                            let session_id = session_id.clone();
                            let mut internal_state = internal_state.lock().await;
                            match internal_state.get_connection(&session_id) {
                                Some(tx) => match tx.send(warp_message.clone()).await {
                                    Err(error) => {
                                        error!("Failed to send message to client: {:?}", error)
                                    }
                                    _ => (),
                                },
                                None => {
                                    error!(
                                        "Could not find connection for session {:?}",
                                        session_id
                                    );
                                }
                            }
                        });

                        futures::future::join_all(send_futures).await;
                    }
                    Err(error) => {
                        error!("Failed to serialize client message: {:?}", error);
                    }
                }
            }
            TransportOutgoingMessage::Purge(_session_id) => todo!(),
        }
        // match message {
        //     Message::
        // }
    }
    Ok(())
}
