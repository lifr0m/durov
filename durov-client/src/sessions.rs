pub mod encoding;
pub mod telethon;

use crate::sessions::encoding::PeerType;
use crate::{tl, Error};
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::SystemTime;
use tl::types::updates::State;

#[derive(Debug)]
pub struct Peer {
    pub id: i64,
    pub typ: PeerType,
    pub access_hash: i64,
    pub username: Option<String>,
}

impl Peer {
    pub fn to_input_peer(&self) -> tl::enums::InputPeer {
        match self.typ {
            PeerType::User => tl::types::InputPeerUser {
                user_id: self.id,
                access_hash: self.access_hash,
            }.into(),

            PeerType::Chat => tl::types::InputPeerChat {
                chat_id: self.id,
            }.into(),

            PeerType::Channel => tl::types::InputPeerChannel {
                channel_id: self.id,
                access_hash: self.access_hash,
            }.into(),
        }
    }

    pub fn to_input_user(&self) -> tl::enums::InputUser {
        assert_eq!(self.typ, PeerType::User);

        tl::types::InputUser {
            user_id: self.id,
            access_hash: self.access_hash,
        }.into()
    }

    pub fn to_input_channel(&self) -> tl::enums::InputChannel {
        assert_eq!(self.typ, PeerType::Channel);

        tl::types::InputChannel {
            channel_id: self.id,
            access_hash: self.access_hash,
        }.into()
    }
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

    async fn get_peer_by_id(&self, id: i64, typ: PeerType) -> Result<Option<Peer>, Error>;

    async fn get_peer_by_username(&self, username: &str) -> Result<Option<Peer>, Error>;

    async fn set_peers<I>(&self, iter: I) -> Result<(), Error>
    where
        I: Iterator<Item = Peer> + Send;

    async fn get_auth(&self) -> Result<Option<Auth>, Error>;

    async fn set_auth(&self, auth: &Auth) -> Result<(), Error>;

    async fn get_states(&self) -> Result<HashMap<i64, State>, Error>;

    async fn set_states(&self, map: &HashMap<i64, State>) -> Result<(), Error>;
}

pub fn get_date() -> i32 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32
}
