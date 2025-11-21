pub mod plain;
pub mod encrypted;
mod msg_id;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("data is too short: {received} bytes, expected at least {expected} bytes")]
    DataTooShort {
        expected: usize,
        received: usize,
    },

    #[error("wrong auth key id: {received}, expected {expected}")]
    WrongAuthKeyId {
        expected: i64,
        received: i64,
    },

    #[error("ignore this message")]
    IgnoreThisMessage,

    #[error("wrong message length: {received} bytes, expected {expected} bytes")]
    WrongMessageLength {
        expected: usize,
        received: usize,
    },
}

pub trait Protocol {
    fn pack(&mut self, message: &[u8]) -> Result<Vec<u8>, Error>;

    fn unpack(&mut self, data: &[u8]) -> Result<Vec<u8>, Error>;
}
