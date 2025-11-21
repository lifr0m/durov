pub mod full;

use async_trait::async_trait;
use thiserror::Error;
use tokio::io;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("seq no mismatch: expected {expected}, received {received}")]
    SeqMismatch {
        expected: i32,
        received: i32,
    },

    #[error("crc mismatch: expected {expected}, received {received}")]
    CrcMismatch {
        expected: i32,
        received: i32,
    },

    #[error("application error code: {0}")]
    Application(i32),
}

#[async_trait]
pub trait Transport {
    async fn send(&mut self, payload: &[u8]) -> Result<(), Error>;

    async fn receive(&mut self) -> Result<Vec<u8>, Error>;
}
