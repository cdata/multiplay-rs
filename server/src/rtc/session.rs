use std::collections::{BTreeMap, BTreeSet};

use crate::rtc::protocol::common::SessionID;

// TODO: These should wrap unbounded channel ports
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub enum Transport {
    Bulk,
    Unordered,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum ConnectionQuality {
    Partial,
    Full,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum SessionStatus {
    Authenticating,
    Connecting,
    Connected(ConnectionQuality),
    Disconnected,
}

pub struct Session {
    pub id: SessionID,
    pub authenticated: bool,
    pub transports: BTreeSet<Transport>,
}

impl Session {
    pub fn new() -> Self {
        Session {
            id: Self::make_id(),
            authenticated: false,
            transports: BTreeSet::new(),
        }
    }

    pub fn make_id() -> SessionID {
        rand::random()
    }

    pub fn has_transport(&self, transport: &Transport) -> bool {
        match self.transports.get(transport) {
            Some(_) => true,
            None => false,
        }
    }

    pub fn add_transport(&mut self, transport: Transport) -> bool {
        match self.has_transport(&transport) {
            false => {
                self.transports.insert(transport);
                true
            }
            true => false,
        }
    }

    pub fn get_status(&self) -> SessionStatus {
        if !self.authenticated {
            SessionStatus::Authenticating
        } else if self.has_transport(&Transport::Bulk) {
            if self.has_transport(&Transport::Unordered) {
                SessionStatus::Connected(ConnectionQuality::Full)
            } else {
                SessionStatus::Connected(ConnectionQuality::Partial)
            }
        } else {
            SessionStatus::Connecting
        }
    }
}

pub type Sessions = BTreeMap<SessionID, Session>;
