use core::fmt::Debug;
use flume::Sender;
use std::net::SocketAddr;

use super::common::SessionID;

/**
 * Network Server Thread <-> Game Thread
 */
#[derive(Debug)]
pub enum OutgoingMessage {
    Send(SessionID, Vec<u8>),
    Broadcast(Vec<u8>),
}

#[derive(Debug)]
pub enum SessionControlMessage {
    Accept(Option<SessionID>),
    Reject(Option<String>),
}

#[derive(Debug)]
pub enum IncomingMessage {
    Solicited(Option<SocketAddr>, Sender<SessionControlMessage>),
    Connected(SessionID),
    Disconnected(SessionID),
    Reconnected(SessionID),
    Departed(SessionID),
    Received(SessionID, Vec<u8>),
}

/**
 * Network Server Thread <-> Network Transport Thread
 */
#[derive(Debug)]
pub enum TransportIncomingMessage {
    Solicited(Option<SocketAddr>, Sender<SessionControlMessage>),
    Connected(SessionID),
    Disconnected(SessionID),
    Received(SessionID, Vec<u8>),
}

#[derive(Debug)]
pub enum TransportOutgoingMessage {
    Send(Vec<SessionID>, Vec<u8>),
    Purge(SessionID),
}
