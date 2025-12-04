pub mod plain;
pub mod encrypted;
pub mod time;
mod constants;
mod checkers;
mod serde;

use durov_tl_types::deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("missing bytes")]
    MissingBytes,

    #[error("deserialize: {0}")]
    Deserialize(#[from] deserialize::Error),

    #[error("auth key id mismatch: expected {expected}, received {received}")]
    AuthKeyIdMismatch {
        expected: i64,
        received: i64,
    },

    #[error("msg key mismatch: expected {expected:?}, received {received:?}")]
    MsgKeyMismatch {
        expected: [u8; 16],
        received: [u8; 16],
    },

    #[error("session id mismatch: expected {expected}, received {received}")]
    SessionIdMismatch {
        expected: i64,
        received: i64,
    },

    #[error("invalid length: {0}")]
    InvalidLength(i32),

    #[error("length too big: expected at most {expected}, received {received}")]
    LengthTooBig {
        expected: usize,
        received: usize,
    },

    #[error("invalid padding length: {0}")]
    InvalidPaddingLength(usize),

    #[error("received invalid msg id: {0}")]
    InvalidMsgId(i64),

    #[error("ignore this message")]
    IgnoreThisMessage,
}
