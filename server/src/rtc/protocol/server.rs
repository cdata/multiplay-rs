use core::fmt::Debug;
use flume::Sender;
use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use super::common::SessionID;

#[derive(Debug)]
pub enum OutgoingMessage {
    Send(SessionID, Vec<u8>),
    Broadcast(Vec<u8>),
}

#[derive(Debug)]
pub enum SessionControlMessage {
    Accept(SessionID),
    Kick(SessionID),
}

#[derive(Debug)]
pub enum IncomingMessage {
    Solicited(SessionID, Option<SocketAddr>, Sender<SessionControlMessage>),
    Connected(SessionID),
    Disconnected(SessionID),
    Reconnected(SessionID),
    Departed(SessionID),
    Received(SessionID, Vec<u8>),
}

#[derive(Debug)]
pub enum TransportIncomingMessage {
    Received(SessionID, Vec<u8>),
    Connected(SessionID),
    Disconnected(SessionID),
}

#[derive(Debug)]
pub enum TransportOutgoingMessage {
    Send(Vec<SessionID>, Vec<u8>),
    Purge(SessionID),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionPayload {
    pub id: SessionID,
}
