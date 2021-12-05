use core::fmt::Debug;
use serde::{Deserialize, Serialize};

use super::common::SessionID;

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    Handshake(SessionID),
    Data(Vec<u8>),
}
