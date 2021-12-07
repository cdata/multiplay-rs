use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use rand::{Fill, RngCore};
use std::sync::Arc;

use async_std::sync::Mutex;
use flume::{Receiver, Sender};
use tokio::{select, task::JoinHandle};

use crate::rtc::session::SessionStatus;

use super::{
    protocol::server::{
        IncomingMessage, OutgoingMessage, SessionControlMessage, TransportIncomingMessage,
        TransportOutgoingMessage,
    },
    session::{Session, Sessions, Transport},
};

/**
 * Common signature for all transports.
 * TODO: This is a trait because it's quite difficult to correctly type
 * a generic async function. This could probably just be a function if
 * I knew how to type it correctly.
 * See also:
 *  - https://stackoverflow.com/questions/65696254/is-it-possible-to-create-a-type-alias-for-async-functions
 *  - https://stackoverflow.com/questions/61167939/return-an-async-function-from-a-function-in-rust
 */
#[async_trait]
pub trait RtcTransport {
    async fn start(
        self,
        shared_sessions: Arc<Mutex<Sessions>>,
        send_to_server: Sender<TransportIncomingMessage>,
        receive_from_server: Receiver<TransportOutgoingMessage>,
    ) -> Result<()>;

    fn get_type(&self) -> Transport;
}

pub static MAX_UNORDERED_DATA_SIZE: usize = 1024usize;

pub struct RtcServer {
    admin_secret: String,
    shared_sessions: Arc<Mutex<Sessions>>,

    game_send_to_server: Sender<OutgoingMessage>,
    game_receive_from_server: Receiver<IncomingMessage>,

    server_send_to_game: Sender<IncomingMessage>,
    server_receive_from_game: Receiver<OutgoingMessage>,
}

impl RtcServer {
    async fn receive_messages_from_transport(
        transport_type: Transport,
        admin_secret: String,
        shared_sessions: Arc<Mutex<Sessions>>,
        send_to_game: Sender<IncomingMessage>,
        receiver: Receiver<TransportIncomingMessage>,
    ) -> Result<()> {
        while let Ok(message) = receiver.recv_async().await {
            let result: Result<()> = match message {
                TransportIncomingMessage::Solicited(socket_addr, secret, transport_auth_sender) => {
                    let (auth_sender, auth_receiver) = flume::unbounded::<SessionControlMessage>();
                    let mut is_admin: bool = false;

                    let authorize_result = match secret {
                        None => send_to_game
                            .send_async(IncomingMessage::Solicited(socket_addr, auth_sender))
                            .await
                            .context("Error requesting authentication for new session"),
                        Some(secret) => match secret == admin_secret {
                            true => {
                                is_admin = true;
                                auth_sender
                                    .send(SessionControlMessage::Accept(None))
                                    .with_context(|| "Failed to send admin acceptance signal")
                            }
                            false => Err(anyhow!("Failed admin authentication attempt")),
                        },
                    };

                    if let Err(_) = authorize_result {
                        return authorize_result;
                    }

                    match auth_receiver
                        .recv_async()
                        .await
                        .context("Error receiving authentication for new session")
                    {
                        Ok(message) => match message {
                            SessionControlMessage::Accept(custom_session_id) => {
                                let mut sessions = shared_sessions.lock().await;
                                let mut session = Session::new();

                                if let Some(session_id) = custom_session_id {
                                    session.id = session_id;
                                }

                                session.admin = is_admin;

                                let id = session.id;

                                sessions.insert(id, session);

                                transport_auth_sender
                                    .send_async(SessionControlMessage::Accept(Some(id)))
                                    .await
                                    .context("Failed to forward session acceptance to transport")
                            }
                            SessionControlMessage::Reject(reason) => {
                                info!("Session request rejected (reason: {:?})", reason);

                                transport_auth_sender
                                    .send_async(SessionControlMessage::Reject(reason))
                                    .await
                                    .context("Failed to forward session rejection to transport")
                            }
                        },
                        Err(err) => Err(err),
                    }
                }
                TransportIncomingMessage::Received(session_id, data) => send_to_game
                    .send_async(IncomingMessage::Received(session_id, data))
                    .await
                    .context("Failed to send 'Received' message to game"),
                TransportIncomingMessage::Connected(session_id, maybe_secret) => {
                    let mut sessions = shared_sessions.lock().await;

                    // TODO: Need to validate that the transport type isn't already
                    // connected; if it is, the connection needs to be rejected!
                    if let Some(session) = sessions.get_mut(&session_id) {
                        match maybe_secret {
                            Some(secret) if session.admin && secret != admin_secret => {
                                Err(anyhow!("Attempt to connect with invalid admin secret"))
                            }
                            None if session.admin => {
                                Err(anyhow!("Attempt to connect as admin without secret"))
                            }
                            _ => {
                                session.transports.insert(transport_type.clone());

                                // As long as the client has a bulk transport, they can be considered
                                // connected. Unordered messages can fall-back to sending over the bulk
                                // transport.
                                if session.has_transport(&Transport::Bulk) {
                                    return send_to_game
                                        .send_async(IncomingMessage::Connected(session_id))
                                        .await
                                        .context("Failed to send 'Connected' message to game");
                                } else {
                                    Ok(())
                                }
                            }
                        }
                    } else {
                        Ok(())
                    }
                }
                TransportIncomingMessage::Disconnected(session_id) => {
                    let mut sessions = shared_sessions.lock().await;
                    // TODO: Do we need a "degraded" connection state when there is no
                    // available unordered transport?
                    if let Some(session) = sessions.get_mut(&session_id) {
                        session.transports.remove(&transport_type);
                        if !session.has_transport(&Transport::Bulk) {
                            return send_to_game
                                .send_async(IncomingMessage::Disconnected(session_id))
                                .await
                                .context("Failed to send 'Disconnected' message to game");
                        }
                    }

                    Ok(())
                }
            };

            if let Err(error) = result {
                error!("{:?}", error);
            }
        }

        Ok(())
    }

    async fn run_receive_message_loop(
        admin_secret: String,
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
                        admin_secret.clone(),
                        shared_sessions.clone(),
                        send_to_game.clone(),
                        receiver,
                    )
                })
                .chain(unordered_transport_receivers.into_iter().map(|receiver| {
                    (
                        Transport::Unordered,
                        admin_secret.clone(),
                        shared_sessions.clone(),
                        send_to_game.clone(),
                        receiver,
                    )
                }))
                .map(
                    |(transport_type, admin_secret, shared_sessions, send_to_game, receiver): (
                        Transport,
                        String,
                        Arc<Mutex<Sessions>>,
                        Sender<IncomingMessage>,
                        Receiver<TransportIncomingMessage>,
                    )| async move {
                        let transport_string = transport_type.to_string();
                        let result = RtcServer::receive_messages_from_transport(
                            transport_type,
                            admin_secret,
                            shared_sessions,
                            send_to_game,
                            receiver,
                        )
                        .await
                        .context(format!(
                            "Error receiving messages from {} transport...",
                            transport_string
                        ));

                        match result {
                            Ok(_) => {
                                info!("Receive loop for {} transport stopped", transport_string)
                            }
                            Err(error) => error!("{:?}", error),
                        };
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

            info!("Got message to send...");

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

            info!("Bulk sending {:?} to {:?}", data, bulk_session_ids);
            info!(
                "Unordered sending {:?} to {:?}",
                data, unordered_session_ids
            );

            let send_futures = unordered_transport_senders
                .iter()
                .filter(|_| unordered_session_ids.len() > 0)
                .map(|sender| {
                    info!("Sending to unordered transport...");
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
                            info!("Sending to bulk transport...");
                            sender.send_async(TransportOutgoingMessage::Send(
                                bulk_session_ids.clone(),
                                data.clone(),
                            ))
                        }),
                );

            match futures::future::try_join_all(send_futures).await {
                _ => info!("Done sending message to transports"),
            };
        }
    }

    pub fn new() -> Self {
        let shared_sessions = Arc::new(Mutex::new(Sessions::new()));
        let (server_send_to_game, game_receive_from_server) = flume::unbounded::<IncomingMessage>();
        let (game_send_to_server, server_receive_from_game) = flume::unbounded::<OutgoingMessage>();

        let mut random_bytes: [u8; 16] = [0; 16];
        rand::thread_rng().fill_bytes(&mut random_bytes);

        RtcServer {
            admin_secret: random_bytes
                .iter()
                .map(|byte| format!("{:x}", byte))
                .collect::<String>(),
            shared_sessions,

            server_send_to_game,
            server_receive_from_game,

            game_send_to_server,
            game_receive_from_server,
        }
    }

    pub fn get_sessions(&self) -> Arc<Mutex<Sessions>> {
        self.shared_sessions.clone()
    }

    pub async fn run<T: RtcTransport + 'static>(self, transports: Vec<T>) -> Result<()> {
        info!("RTC server admin secret: {}", self.admin_secret);

        let server_receive_from_game = self.server_receive_from_game.clone();
        let server_send_to_game = self.server_send_to_game.clone();
        let shared_sessions = self.shared_sessions.clone();

        let mut bulk_transport_senders: Vec<Sender<TransportOutgoingMessage>> = Vec::new();
        let mut unordered_transport_senders: Vec<Sender<TransportOutgoingMessage>> = Vec::new();

        let mut bulk_transport_receivers: Vec<Receiver<TransportIncomingMessage>> = Vec::new();
        let mut unordered_transport_receivers: Vec<Receiver<TransportIncomingMessage>> = Vec::new();

        let mut transport_handles: Vec<JoinHandle<Result<()>>> = Vec::new();

        // Spin up a task thread for each transport, and establish
        // communication over dedicated mpsc channels organized by
        // transport type
        for transport in transports.into_iter() {
            // let (transport_type, start_fn) = transport;
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
            self.admin_secret.clone(),
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

        select! {
            _ = send_message_loop =>
                Err(anyhow!("Send message loop stopped")),

            _ = receive_message_loop =>
                Err(anyhow!("Receive message loop stopped")),

            result = transports_join => {
                match result {
                    Err(error) =>
                        Err(error).context("Transport thread join error"),
                    Ok(results) => {
                        for result in results {
                            if let Err(error) = result {
                                return Err(error)
                                    .context("Transport thread joined because of error in transport")
                            }
                        }
                        Ok(())
                    }
                }
            },
        }
    }

    pub fn game_channel(&self) -> (Sender<OutgoingMessage>, Receiver<IncomingMessage>) {
        (
            self.game_send_to_server.clone(),
            self.game_receive_from_server.clone(),
        )
    }
}
