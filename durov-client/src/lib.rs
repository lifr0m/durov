pub mod client;
pub mod srp;
mod datacenters;

pub use durov_tl_types::schemas::api as tl;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("mt client: {0}")]
    MtClient(durov_mtclient::Error),

    #[error("srp: {0}")]
    Srp(#[from] srp::Error),

    #[error("invalid rpc error: {0}")]
    InvalidRpcError(String),

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

pub struct Config {
    pub api_id: i32,
    pub api_hash: String,
    pub device_model: String,
    pub system_version: String,
    pub app_version: String,
    pub system_lang_code: String,
    pub lang_pack: String,
    pub lang_code: String,
    pub params: Option<tl::enums::JsonValue>,
    pub use_compression: bool,
}

impl Config {
    pub fn new(api_id: i32, api_hash: String) -> Self {
        Self {
            api_id,
            api_hash,
            device_model: String::from("Unknown"),
            system_version: String::from("Unknown"),
            app_version: String::from("Unknown"),
            system_lang_code: String::from("en"),
            lang_pack: String::new(),
            lang_code: String::from("en"),
            params: None,
            use_compression: true,
        }
    }
}
