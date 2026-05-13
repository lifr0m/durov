use crate::client::Client;
use crate::sessions::encoding::PeerType;
use crate::sessions::peer::Peer;
use crate::sessions::Session;
use crate::{tl, Error};
use durov_mtproto::transports::Transport;

impl<T: Transport, S: Session> Client<T, S>
where
    T: Send + 'static,
{
    pub async fn resolve_username(&self, username: &str) -> Result<Peer, Error> {
        if let Some(peer) = self.session.get_peer_by_username(username).await? {
            return Ok(peer);
        }

        let resolved = self.call(tl::functions::contacts::ResolveUsername {
            username: username.to_string(),
            referer: None,
        }).await?;
        let tl::enums::contacts::ResolvedPeer::ResolvedPeer(resolved) = resolved;

        self.apply_peers(&resolved.chats, &resolved.users).await?;

        let peer = self.session.get_peer_by_username(username).await?
            .expect("server did not send peer with requested username");
        Ok(peer)
    }

    pub(super) async fn apply_peers(&self, chats: &[tl::enums::Chat], users: &[tl::enums::User])
        -> Result<(), Error>
    {
        let chat_iter = chats.iter()
            .filter_map(|chat| match chat {
                tl::enums::Chat::Channel(channel) => {
                    channel.access_hash.map(|access_hash| Peer {
                        id: channel.id,
                        typ: PeerType::Channel,
                        access_hash,
                        username: channel.username.clone(),
                    })
                }
                tl::enums::Chat::ChannelForbidden(channel) => {
                    Some(Peer {
                        id: channel.id,
                        typ: PeerType::Channel,
                        access_hash: channel.access_hash,
                        username: None,
                    })
                }
                _ => None,
            });

        let user_iter = users.iter()
            .filter_map(|user| match user {
                tl::enums::User::User(user) => {
                    user.access_hash.map(|access_hash| Peer {
                        id: user.id,
                        typ: PeerType::User,
                        access_hash,
                        username: user.username.clone(),
                    })
                }
                _ => None,
            });

        let iter = chat_iter.chain(user_iter);
        self.session.set_peers(iter).await?;

        Ok(())
    }
}
