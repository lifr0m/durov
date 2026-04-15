use crate::sessions::encoding::PeerType;
use crate::sessions::{Peer, Session};
use crate::{tl, Error};

pub async fn apply_peers<S: Session>(
    session: &S,
    chat_list: &[tl::enums::Chat],
    user_list: &[tl::enums::User],
) -> Result<(), Error> {
    let chat_iter = chat_list.iter()
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

    let user_iter = user_list.iter()
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
    session.set_peers(iter).await?;

    Ok(())
}
