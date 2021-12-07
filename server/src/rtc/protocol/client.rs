use core::fmt::Debug;
use serde::{Deserialize, Serialize};

use super::common::SessionID;

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    // The first field is a session ID, for reconnecting after connection loss
    // The second field is for the admin secret, used when introspecting server state
    Handshake(Option<SessionID>, Option<String>),
    Data(Vec<u8>),
    Ping(u32),
    Pong(u32),
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct SessionPayload {
//     pub id: SessionID,
// }
