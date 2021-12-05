use std::collections::HashMap;
use std::{net::SocketAddr, sync::Arc};

use async_std::sync::Mutex;
use bimap::BiMap;
use flume::{Receiver, Sender};
use futures::stream::SplitSink;
use futures::SinkExt;
use tokio::task::{self, JoinHandle};
use warp::ws::WebSocket;
use warp::Filter;
use warp::{self, ws::Message as WarpMessage};

use futures_util::StreamExt;

use crate::rtc::protocol::client::Message;
use crate::rtc::session::Transport;
use crate::{
    rtc::protocol::server::{IncomingMessage, OutgoingMessage},
    rtc::session::Sessions,
};

use super::protocol::common::SessionID;

type Connections = HashMap<SocketAddr, SplitSink<WebSocket, WarpMessage>>;
type AddressSessions = BiMap<SocketAddr, SessionID>;

struct State {
    connections: Connections,
    address_sessions: AddressSessions,
}

impl State {
    pub fn new() -> Self {
        State {
            connections: Connections::new(),
            address_sessions: AddressSessions::new(),
        }
    }
}

async fn get_session_id(shared_state: Arc<Mutex<State>>, address: SocketAddr) -> Option<SessionID> {
    let state = shared_state.lock().await;
    if let Some(session_id) = state.address_sessions.get_by_left(&address) {
        Some(*session_id)
    } else {
        None
    }
}

#[derive(Debug)]
enum SendError {
    MissingAddress,
    MissingConnection,
    WebSocketError(warp::Error),
}

async fn send_to_session(
    shared_state: Arc<Mutex<State>>,
    session_id: &SessionID,
    buffer: &Vec<u8>,
) -> Result<(), SendError> {
    match shared_state
        .lock()
        .await
        .address_sessions
        .get_by_right(session_id)
    {
        Some(address) => match shared_state.lock().await.connections.get_mut(&address) {
            Some(tx) => {
                if let Err(error) = tx.send(WarpMessage::binary(buffer.as_slice())).await {
                    Err(SendError::WebSocketError(error))
                } else {
                    Ok(())
                }
            }
            None => Err(SendError::MissingConnection),
        },
        None => {
            warn!("No Web Socket address for session {:?}", session_id);
            Err(SendError::MissingAddress)
        }
    }
}

async fn handle_outgoing_messages(
    shared_state: Arc<Mutex<State>>,
    receive_from_server: Receiver<OutgoingMessage>,
) {
    while let Ok(message) = receive_from_server.recv_async().await {
        let (session_ids, message) = match message {
            OutgoingMessage::Send(session_id, message) => (vec![session_id], message),
            OutgoingMessage::Broadcast(message) => (
                shared_state
                    .lock()
                    .await
                    .address_sessions
                    .right_values()
                    .map(|session_id| *session_id)
                    .collect(),
                message,
            ),
        };

        let messages_are_sent = session_ids
            .iter()
            .map(|session_id| send_to_session(shared_state.clone(), session_id, &message));

        futures::future::join_all(messages_are_sent)
            .await
            .iter()
            .zip(session_ids)
            .for_each(|(result, session_id)| {
                match result {
                    Err(SendError::WebSocketError(error)) => {
                        error!(
                            "Web Socket error when sending message to {:?}: {:?}",
                            session_id, error
                        );
                        // TODO: Update state to reflect error condition
                    }
                    Err(error) => {
                        error!(
                            "Error sending to Web Socket client {:?}: {:?}",
                            session_id, error
                        );
                        // TODO: Update state to reflect error condition
                    }
                    _ => {}
                }
            });
    }
}

async fn handle_incoming_messages(
    shared_sessions: Arc<Mutex<Sessions>>,
    shared_state: Arc<Mutex<State>>,
    address: SocketAddr,
    send_to_server: Sender<IncomingMessage>,
) {
    let with_shared_state = warp::any().map(move || shared_state.clone());
    let with_shared_sessions = warp::any().map(move || shared_sessions.clone());
    let with_send_to_server = warp::any().map(move || send_to_server.clone());

    let routes = warp::any()
        .and(warp::ws())
        .and(with_shared_state)
        .and(with_shared_sessions)
        .and(with_send_to_server)
        .and(warp::filters::addr::remote())
        .map(
            |ws: warp::ws::Ws, shared_state: Arc<Mutex<State>>, shared_sessions: Arc<Mutex<Sessions>>, send_to_server: Sender<IncomingMessage>, maybe_address| {
                ws.on_upgrade(move |websocket| async move {
                    match maybe_address {
                        Some(address) => {
                            let (tx, mut rx) = websocket.split();

                            let mut state = shared_state.lock().await;
                            state.connections.insert(address, tx);

                            while let Some(result) = rx.next().await {
                                match result {
                                    Ok(message) => {
                                        if !message.is_binary() {
                                            warn!(
                                                "Received non-binary Web Socket message from {:?}",
                                                address
                                            );
                                            continue;
                                        }

                                        match get_session_id(shared_state.clone(), address).await {
                                            Some(session_id) => {

                                              match serde_cbor::de::from_slice(message.as_bytes()) {
                                                Ok(Message::Data(message)) => {
                                                  if let Err(error) = send_to_server.send_async(IncomingMessage::Received(session_id, message)).await {
                                                    error!("Error forwarding Web Socket message: {:?}", error);
                                                  }
                                                },
                                                Err(error) => error!("Web Socket payload deserialization error: {:?}", error),
                                                _ => warn!("Unexpected Web Socket message payload")
                                              }
                                            }
                                            None => {

                                              match serde_cbor::de::from_slice(message.as_bytes()) {
                                                Ok(Message::Handshake(session_id)) => {
                                                  match shared_sessions.lock().await.get_mut(&session_id) {
                                                      Some(session) => {
                                                        let mut state = shared_state.lock().await;
                                                        state.address_sessions.insert(address, session_id);
                                                        session.transports.insert(Transport::Bulk);
                                                      },
                                                      None => {
                                                          error!("No session found for {:?}", session_id);
                                                      }
                                                  }


                                                  // TODO: update sessions...

                                                },
                                                Err(error) => error!("Web Socket payload deserialization error: {:?}", error),
                                                _ => warn!("Unexpected Web Socket message payload")
                                              }

                                            }
                                        }
                                    }
                                    Err(error) => {
                                        error!("Web Socket receive error: {:?}", error);
                                    }
                                }
                            }
                        }
                        _ => {
                            warn!("Skipping connection from unknown address");
                        }
                    };
                })
            },
        );

    warp::serve(routes).run(address).await;
}

async fn serve_websockets_task(
    shared_sessions: Arc<Mutex<Sessions>>,
    shared_state: Arc<Mutex<State>>,
    address: SocketAddr,
    send_to_server: Sender<IncomingMessage>,
    receive_from_server: Receiver<OutgoingMessage>,
) {
    info!("WAK WAK");
    tokio::join!(
        handle_incoming_messages(
            shared_sessions.clone(),
            shared_state.clone(),
            address.clone(),
            send_to_server.clone()
        ),
        handle_outgoing_messages(shared_state.clone(), receive_from_server)
    );
}

pub fn serve_websockets(
    shared_sessions: Arc<Mutex<Sessions>>,
    address: SocketAddr,
    send_to_server: Sender<IncomingMessage>,
    receive_from_server: Receiver<OutgoingMessage>,
) -> JoinHandle<()> {
    let shared_state = Arc::new(Mutex::new(State::new()));

    info!("Starting Web Socket server...");

    task::spawn(serve_websockets_task(
        shared_sessions.clone(),
        shared_state.clone(),
        address,
        send_to_server.clone(),
        receive_from_server.clone(),
    ))
}
