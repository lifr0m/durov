pub mod plain;
pub mod encrypted;
pub mod config;
pub mod proxy;
mod tcp;

use thiserror::Error;
use tokio::io;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("transport: {0}")]
    Transport(#[from] durov_mtproto::transports::Error),

    #[error("protocol: {0}")]
    Protocol(#[from] durov_mtproto::protocols::Error),

    #[error("auth: {0}")]
    Auth(#[from] durov_crypto::auth::Error),

    #[error("auth failed after several unsuccessful attempts")]
    AuthFailed,

    #[error("connection closed")]
    Connection,

    #[error("rpc error: code {code}, message: {message}")]
    RpcError {
        code: i32,
        message: String,
    },
}
