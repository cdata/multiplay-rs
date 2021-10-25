use std::collections::{HashMap, HashSet};

use crate::rtc::protocol::SessionID;

// TODO: These should wrap unbounded channel ports
#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum Channel {
    WebSocket,
    WebRTC,
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum ConnectionQuality {
    Partial,
    Full,
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum SessionStatus {
    Authenticating,
    Connecting,
    Connected(ConnectionQuality),
    Disconnected,
}

pub struct Session {
    pub id: SessionID,
    pub authenticated: bool,
    pub channels: HashSet<Channel>,
}

impl Session {
    pub fn new() -> Self {
        Session {
            id: Self::make_id(),
            authenticated: false,
            channels: HashSet::new(),
        }
    }

    pub fn make_id() -> SessionID {
        rand::random()
    }

    pub fn has_channel(&self, channel: Channel) -> bool {
        match self.channels.get(&channel) {
            Some(_) => true,
            None => false,
        }
    }

    pub fn get_status(&self) -> SessionStatus {
        if !self.authenticated {
            SessionStatus::Authenticating
        } else if self.has_channel(Channel::WebSocket) {
            if self.has_channel(Channel::WebRTC) {
                SessionStatus::Connected(ConnectionQuality::Full)
            } else {
                SessionStatus::Connected(ConnectionQuality::Partial)
            }
        } else {
            SessionStatus::Connecting
        }
    }
}

pub type Sessions = HashMap<SessionID, Session>;
