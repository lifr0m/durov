pub mod telethon;

use crate::{tl, Error};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct Peer {
    pub id: i64,
    pub access_hash: i64,
}

pub struct Auth {
    pub dc_id: i32,
    pub dc_host: String,
    pub dc_port: u16,
    pub auth_key: [u8; 256],
}

#[async_trait]
pub trait Session: Sized {
    async fn connect(info: &str) -> Result<Self, Error>;

    async fn init(&self) -> Result<(), Error>;

    async fn get_peer(&self, username: &str) -> Result<Option<Peer>, Error>;

    async fn set_peer(&self, peer: Peer, username: &str) -> Result<(), Error>;

    async fn get_auth(&self) -> Result<Option<Auth>, Error>;

    async fn set_auth(&self, auth: Auth) -> Result<(), Error>;

    async fn list_states(&self) -> Result<Vec<tl::types::updates::State>, Error>;

    async fn set_state(&self, id: i64, state: tl::types::updates::State) -> Result<(), Error>;
}

pub fn get_date() -> i32 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32
}
