pub mod client;
pub mod srp;
pub mod config;
pub mod sessions;
mod datacenters;

pub use durov_tl_types::schemas::api as tl;
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("mt client: {0}")]
    MtClient(durov_mtclient::Error),

    #[error("srp: {0}")]
    Srp(#[from] srp::Error),

    #[error("database: {0}")]
    Database(#[from] sqlx::Error),

    #[error("rpc error: code {code}, message: {message}")]
    RpcError {
        code: i32,
        message: String,
    },

    #[error("invalid rpc error: {0}")]
    InvalidRpcError(String),
}

impl From<durov_mtclient::Error> for Error {
    fn from(err: durov_mtclient::Error) -> Self {
        match err {
            durov_mtclient::Error::RpcError { code, message } => Self::RpcError { code, message },
            _ => Self::MtClient(err),
        }
    }
}

impl Error {
    pub fn is(&self, status: i32, pattern: &str) -> bool {
        match self {
            Self::RpcError { code, message } => *code == status
                && message.starts_with(pattern),
            _ => false,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::RpcError { message, .. } => message,
            _ => unreachable!("error should be rpc error"),
        }
    }

    pub fn parse<T: FromStr>(&self, pattern: &str) -> Result<T, Error> {
        let index = pattern.split("_")
            .position(|ch| ch == "X")
            .expect("pattern should contain X");
        self.message()
            .split("_")
            .nth(index)
            .ok_or_else(|| Error::InvalidRpcError(self.message().to_string()))?
            .parse()
            .map_err(|_| Error::InvalidRpcError(self.message().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        for (status, message, pattern, result) in [
            (303, "USER_MIGRATE_3", "USER_MIGRATE_X", 3),
            (400, "FILE_PART_42_MISSING", "FILE_PART_X_MISSING", 42),
        ] {
            let err = Error::RpcError {
                code: status,
                message: message.to_string(),
            };
            assert_eq!(err.parse::<i32>(pattern).unwrap(), result);
        }
    }
}
