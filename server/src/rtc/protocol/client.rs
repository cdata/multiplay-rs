use core::fmt::Debug;
use serde::{Deserialize, Serialize};

use super::common::SessionID;

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    Handshake(Option<SessionID>),
    Data(Vec<u8>),
    Ping(u32),
    Pong(u32),
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct SessionPayload {
//     pub id: SessionID,
// }
