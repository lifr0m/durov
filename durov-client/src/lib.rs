pub mod client;
pub mod config;
pub mod sessions;
pub mod error;
mod manager;
mod datacenters;

pub use durov_tl_types::schemas::api as tl;
use thiserror::Error;
use tokio::io;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    
    #[error("mtproto client: {0}")]
    MtClient(durov_mtclient::Error),

    #[error("srp: {0}")]
    Srp(#[from] durov_crypto::srp::Error),

    #[error("database: {0}")]
    Database(#[from] sqlx::Error),

    #[error("rpc error: code {code}, message: {message}")]
    RpcError {
        code: i32,
        message: String,
    },
}

impl From<durov_mtclient::Error> for Error {
    fn from(err: durov_mtclient::Error) -> Self {
        match err {
            durov_mtclient::Error::RpcError { code, message } => Self::RpcError { code, message },
            _ => Self::MtClient(err),
        }
    }
}
