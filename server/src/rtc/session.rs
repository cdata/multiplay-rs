use std::collections::{BTreeMap, BTreeSet};

use crate::rtc::protocol::common::SessionID;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub enum Transport {
    Bulk,
    Unordered,
}

impl ToString for Transport {
    fn to_string(&self) -> String {
        String::from(match self {
            Transport::Bulk => "bulk",
            Transport::Unordered => "unordered",
        })
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum ConnectionQuality {
    Partial,
    Full,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum SessionStatus {
    Authenticating,
    Connected(ConnectionQuality),
    Disconnected,
}

pub struct Session {
    pub id: SessionID,
    pub authenticated: bool,
    pub admin: bool,
    pub transports: BTreeSet<Transport>,
}

impl Session {
    pub fn new() -> Self {
        Session {
            id: Self::make_id(),
            authenticated: false,
            admin: false,
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
            SessionStatus::Disconnected
        }
    }
}

pub type Sessions = BTreeMap<SessionID, Session>;
