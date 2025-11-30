pub mod full;

use durov_tl_types::buffer::Buffer;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("missing {0} bytes")]
    MissingBytes(usize),

    #[error("application code: {0}")]
    Application(i32),

    #[error("length too small: expected at least {expected}, received {received}")]
    LengthTooSmall {
        expected: usize,
        received: usize,
    },

    #[error("seq mismatch: expected {expected}, received {received}")]
    SeqMismatch {
        expected: i32,
        received: i32,
    },

    #[error("crc mismatch: expected {expected}, received {received}")]
    CrcMismatch {
        expected: i32,
        received: i32,
    },
}

pub trait Transport: Default {
    fn pack(&mut self, buf: &mut Buffer);

    fn unpack(&mut self, buf: &mut Buffer) -> Result<(), Error>;
}
