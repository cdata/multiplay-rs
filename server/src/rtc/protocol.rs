use core::fmt::Debug;
use flume::Sender;
use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

pub type SessionID = u32;

#[derive(Debug)]
pub enum OutgoingMessage<T>
where
    T: Deserialize<'static> + Serialize + Send + Sync + Debug,
{
    Send(SessionID, T),
    Broadcast(T),
}

#[derive(Debug)]
pub enum SessionControlMessage {
    Accept(SessionID),
    Kick(SessionID),
}

#[derive(Debug)]
pub enum IncomingMessage<T>
where
    T: Deserialize<'static> + Serialize + Send + Sync + Debug,
{
    Solicited(SessionID, Option<SocketAddr>, Sender<SessionControlMessage>),
    Connected(SessionID),
    Disconnected(SessionID),
    Reconnected(SessionID),
    Departed(SessionID),
    Received(SessionID, T),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionPayload {
    pub id: SessionID,
}
