use crate::sessions::encoding::PeerType;
use crate::tl;

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
