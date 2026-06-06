pub mod encoding;
pub mod peer;
#[cfg(feature = "session-telethon")]
pub mod telethon;

use crate::sessions::encoding::PeerType;
use crate::sessions::peer::Peer;
use crate::{tl, Error};
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::SystemTime;
use tl::types::updates::State;

#[async_trait]
pub trait Session {
    async fn connect(info: &str) -> Result<Self, Error>
    where
        Self: Sized;

    async fn init(&self) -> Result<(), Error>;

    async fn get_peer_by_id(&self, id: i64, typ: PeerType) -> Result<Option<Peer>, Error>;

    async fn get_peer_by_username(&self, username: &str) -> Result<Option<Peer>, Error>;

    async fn set_peers<I>(&self, iter: I) -> Result<(), Error>
    where
        I: Iterator<Item = Peer> + Send;

    async fn get_main_dc(&self) -> Result<Option<i32>, Error>;

    async fn set_main_dc(&self, dc_id: i32) -> Result<(), Error>;

    async fn get_auth_key(&self, dc_id: i32) -> Result<Option<[u8; 256]>, Error>;

    async fn set_auth_key(&self, dc_id: i32, auth_key: [u8; 256]) -> Result<(), Error>;

    async fn get_states(&self) -> Result<HashMap<i64, State>, Error>;

    async fn set_states(&self, map: &HashMap<i64, State>) -> Result<(), Error>;
}

pub fn get_date() -> i32 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32
}
