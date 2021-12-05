use async_trait::async_trait;
use std::{net::SocketAddr, sync::Arc};

use async_std::sync::Mutex;
use flume::{Receiver, Sender};
use tokio::{select, task::JoinHandle};

use crate::rtc::session::SessionStatus;

use super::{
    api::sessions::sessions_api,
    protocol::server::{
        IncomingMessage, OutgoingMessage, TransportIncomingMessage, TransportOutgoingMessage,
    },
    session::{Sessions, Transport},
};

#[derive(Debug)]
pub enum TransportError {
    UnexpectedHalt,
    NoAddress,
    NoConnection,
    SendFailed,
}

#[derive(Debug)]
pub enum ServerError {
    SendFailed,
}

#[async_trait]
pub trait RtcTransport: Clone {
    async fn start(
        self,
        shared_sessions: Arc<Mutex<Sessions>>,
        send_to_server: Sender<TransportIncomingMessage>,
        receive_from_server: Receiver<TransportOutgoingMessage>,
    ) -> Result<(), TransportError>;

    fn get_type(&self) -> Transport;
}

pub static MAX_UNORDERED_DATA_SIZE: usize = 1024usize;

pub struct RtcServer {
    shared_sessions: Arc<Mutex<Sessions>>,

    game_send_to_server: Sender<OutgoingMessage>,
    game_receive_from_server: Receiver<IncomingMessage>,

    server_send_to_game: Sender<IncomingMessage>,
    server_receive_from_game: Receiver<OutgoingMessage>,
}

impl RtcServer {
    async fn run_receive_message_loop(
        shared_sessions: Arc<Mutex<Sessions>>,
        send_to_game: Sender<IncomingMessage>,
        bulk_transport_receivers: Vec<Receiver<TransportIncomingMessage>>,
        unordered_transport_receivers: Vec<Receiver<TransportIncomingMessage>>,
    ) {
        futures::future::join_all(
            bulk_transport_receivers
                .into_iter()
                .map(|receiver| {
                    (
                        Transport::Bulk,
                        shared_sessions.clone(),
                        send_to_game.clone(),
                        receiver,
                    )
                })
                .chain(unordered_transport_receivers.into_iter().map(|receiver| {
                    (
                        Transport::Unordered,
                        shared_sessions.clone(),
                        send_to_game.clone(),
                        receiver,
                    )
                }))
                .map(
                    |(transport_type, shared_sessions, send_to_game, receiver): (
                        Transport,
                        Arc<Mutex<Sessions>>,
                        Sender<IncomingMessage>,
                        Receiver<TransportIncomingMessage>,
                    )| async move {
                        while let Ok(message) = receiver.recv_async().await {
                            let mut sessions = shared_sessions.lock().await;

                            match message {
                                TransportIncomingMessage::Connected(session_id) => {
                                    // TODO: Need to validate that the transport type isn't already
                                    // connected; if it is, the connection needs to be rejected!
                                    if let Some(session) = sessions.get_mut(&session_id) {
                                        session.transports.insert(transport_type.clone());

                                        // As long as the client has a bulk transport, they can be considered
                                        // connected. Unordered messages can fall-back to sending over the bulk
                                        // transport.
                                        if session.has_transport(&Transport::Bulk) {
                                            send_to_game
                                                .send_async(IncomingMessage::Connected(session_id))
                                                .await;
                                        }
                                    }
                                }
                                TransportIncomingMessage::Disconnected(session_id) => {
                                    // TODO: Do we need a "degraded" connection state when there is no
                                    // available unordered transport?
                                    if let Some(session) = sessions.get_mut(&session_id) {
                                        session.transports.remove(&transport_type);
                                        if !session.has_transport(&Transport::Bulk) {
                                            send_to_game
                                                .send_async(IncomingMessage::Disconnected(
                                                    session_id,
                                                ))
                                                .await;
                                        }
                                    }
                                }
                                TransportIncomingMessage::Received(session_id, data) => {
                                    send_to_game
                                        .send_async(IncomingMessage::Received(session_id, data))
                                        .await;
                                }
                            }
                        }
                    },
                ),
        )
        .await;
    }

    async fn run_send_message_loop(
        shared_sessions: Arc<Mutex<Sessions>>,
        receive_from_game: Receiver<OutgoingMessage>,
        bulk_transport_senders: Vec<Sender<TransportOutgoingMessage>>,
        unordered_transport_senders: Vec<Sender<TransportOutgoingMessage>>,
    ) {
        while let Ok(message) = receive_from_game.recv_async().await {
            let sessions = shared_sessions.lock().await;

            let (session_ids, data) = match message {
                OutgoingMessage::Broadcast(data) => (sessions.keys().copied().collect(), data),
                OutgoingMessage::Send(session_id, data) => (vec![session_id], data),
            };

            let preferred_transport = match data.len() > MAX_UNORDERED_DATA_SIZE {
                true => Transport::Bulk,
                false => Transport::Unordered,
            };

            let (unordered_session_ids, bulk_session_ids): (Vec<u32>, Vec<u32>) = session_ids
                .iter()
                // Only sessions that have a connected status should receive messages
                .filter(|session_id| match sessions.get(session_id) {
                    Some(session) => match session.get_status() {
                        SessionStatus::Connected(_) => true,
                        _ => false,
                    },
                    _ => false,
                })
                // Split sessions between those that should be messaged over
                // a bulk transport and those that can receive unordered,
                // small payloads
                .partition(|session_id| match sessions.get(session_id) {
                    Some(session) => {
                        preferred_transport == Transport::Unordered
                            && session.has_transport(&Transport::Unordered)
                    }
                    _ => false,
                });

            let send_futures = unordered_transport_senders
                .iter()
                .filter(|_| unordered_session_ids.len() > 0)
                .map(|sender| {
                    sender.send_async(TransportOutgoingMessage::Send(
                        unordered_session_ids.clone(),
                        data.clone(),
                    ))
                })
                .chain(
                    bulk_transport_senders
                        .iter()
                        .filter(|_| bulk_session_ids.len() > 0)
                        .map(|sender| {
                            sender.send_async(TransportOutgoingMessage::Send(
                                bulk_session_ids.clone(),
                                data.clone(),
                            ))
                        }),
                );

            match futures::future::try_join_all(send_futures).await {
                _ => info!("Done"),
            };
        }

        // fn run_http_session_api(shared_sessions: Arc<Mutex<Sessions>>)
    }

    pub fn new() -> Self {
        let shared_sessions = Arc::new(Mutex::new(Sessions::new()));
        let (server_send_to_game, game_receive_from_server) = flume::unbounded::<IncomingMessage>();
        let (game_send_to_server, server_receive_from_game) = flume::unbounded::<OutgoingMessage>();

        RtcServer {
            shared_sessions,

            server_send_to_game,
            server_receive_from_game,

            game_send_to_server,
            game_receive_from_server,
        }
    }

    pub async fn run<T>(
        self,
        api_address: SocketAddr,
        transports: Vec<T>,
    ) -> Result<(), ServerError>
    where
        T: RtcTransport + 'static,
    {
        let server_receive_from_game = self.server_receive_from_game.clone();
        let server_send_to_game = self.server_send_to_game.clone();
        let shared_sessions = self.shared_sessions.clone();

        let mut bulk_transport_senders: Vec<Sender<TransportOutgoingMessage>> = Vec::new();
        let mut unordered_transport_senders: Vec<Sender<TransportOutgoingMessage>> = Vec::new();

        let mut bulk_transport_receivers: Vec<Receiver<TransportIncomingMessage>> = Vec::new();
        let mut unordered_transport_receivers: Vec<Receiver<TransportIncomingMessage>> = Vec::new();

        let mut transport_handles: Vec<JoinHandle<Result<(), TransportError>>> = Vec::new();

        for transport in transports.into_iter() {
            let (server_send_to_transport, transport_receive_from_server) =
                flume::unbounded::<TransportOutgoingMessage>();
            let (transport_send_to_server, server_receive_from_transport) =
                flume::unbounded::<TransportIncomingMessage>();

            let (senders, receivers) = match transport.get_type() {
                Transport::Bulk => (&mut bulk_transport_senders, &mut bulk_transport_receivers),
                Transport::Unordered => (
                    &mut unordered_transport_senders,
                    &mut unordered_transport_receivers,
                ),
            };

            senders.push(server_send_to_transport);
            receivers.push(server_receive_from_transport);

            transport_handles.push(tokio::task::spawn(transport.start(
                shared_sessions.clone(),
                transport_send_to_server,
                transport_receive_from_server,
            )));
        }

        let receive_message_loop = RtcServer::run_receive_message_loop(
            shared_sessions.clone(),
            server_send_to_game.clone(),
            bulk_transport_receivers,
            unordered_transport_receivers,
        );

        let send_message_loop = RtcServer::run_send_message_loop(
            shared_sessions.clone(),
            server_receive_from_game,
            bulk_transport_senders,
            unordered_transport_senders,
        );

        let transports_join = futures::future::try_join_all(transport_handles);

        let http_api = sessions_api(shared_sessions.clone(), server_send_to_game.clone());
        let http_api_handle = tokio::task::spawn(warp::serve(http_api).run(api_address));

        select! {
            _ = send_message_loop => {
                println!("Send message loop stopped");
            },

            _ = receive_message_loop => {
                println!("Receive message loop stopped");
            },

            _ = transports_join => {
                println!("Transport threads joined");
            },

            _ = http_api_handle => {
                println!("HTTP session API stopped");
            }
        }

        Ok(())
    }

    pub fn game_channel(&self) -> (Sender<OutgoingMessage>, Receiver<IncomingMessage>) {
        (
            self.game_send_to_server.clone(),
            self.game_receive_from_server.clone(),
        )
    }
}
