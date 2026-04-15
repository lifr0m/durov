use crate::tl;

pub fn convert_new_message(message: tl::enums::Message) -> tl::enums::Update {
    if match &message {
        tl::enums::Message::MessageEmpty(message) => match &message.peer_id {
            Some(peer) => is_common_peer(peer),
            None => panic!("message doesn't have peer information"),
        }
        tl::enums::Message::Message(message) => is_common_peer(&message.peer_id),
        tl::enums::Message::MessageService(message) => is_common_peer(&message.peer_id),
    } {
        tl::types::UpdateNewMessage {
            message,
            pts: 0,
            pts_count: 0,
        }.into()
    } else {
        tl::types::UpdateNewChannelMessage {
            message,
            pts: 0,
            pts_count: 0,
        }.into()
    }
}

pub fn convert_new_encrypted_message(message: tl::enums::EncryptedMessage) -> tl::enums::Update {
    tl::types::UpdateNewEncryptedMessage {
        message,
        qts: 0,
    }.into()
}

pub fn convert_updates(updates: tl::types::Updates) -> tl::types::UpdatesCombined {
    tl::types::UpdatesCombined {
        updates: updates.updates,
        users: updates.users,
        chats: updates.chats,
        date: updates.date,
        seq_start: updates.seq,
        seq: updates.seq,
    }
}

pub fn convert_update_short(update: tl::types::UpdateShort) -> tl::types::Updates {
    tl::types::Updates {
        updates: vec![update.update],
        users: vec![],
        chats: vec![],
        date: update.date,
        seq: 0,
    }
}

pub fn convert_update_short_message(update: tl::types::UpdateShortMessage) -> tl::types::UpdateShort {
    tl::types::UpdateShort {
        update: tl::types::UpdateNewMessage {
            message: tl::types::Message {
                out: update.out,
                mentioned: update.mentioned,
                media_unread: update.media_unread,
                silent: update.silent,
                post: false,
                from_scheduled: false,
                legacy: false,
                edit_hide: false,
                pinned: false,
                noforwards: false,
                invert_media: false,
                offline: false,
                video_processing_pending: false,
                paid_suggested_post_stars: false,
                paid_suggested_post_ton: false,
                id: update.id,
                from_id: None,
                from_boosts_applied: None,
                peer_id: tl::types::PeerUser {
                    user_id: update.user_id,
                }.into(),
                saved_peer_id: None,
                fwd_from: update.fwd_from,
                via_bot_id: update.via_bot_id,
                via_business_bot_id: None,
                reply_to: update.reply_to,
                date: update.date,
                message: update.message,
                media: None,
                reply_markup: None,
                entities: update.entities,
                views: None,
                forwards: None,
                replies: None,
                edit_date: None,
                post_author: None,
                grouped_id: None,
                reactions: None,
                restriction_reason: None,
                ttl_period: update.ttl_period,
                quick_reply_shortcut_id: None,
                effect: None,
                factcheck: None,
                report_delivery_until_date: None,
                paid_message_stars: None,
                suggested_post: None,
                schedule_repeat_period: None,
                summary_from_language: None,
                from_rank: None,
            }.into(),
            pts: update.pts,
            pts_count: update.pts_count,
        }.into(),
        date: update.date,
    }
}

pub fn convert_update_short_sent_message(update: tl::types::UpdateShortSentMessage) -> tl::types::UpdateShort {
    tl::types::UpdateShort {
        update: tl::types::UpdateNewMessage {
            message: tl::types::MessageEmpty {
                id: update.id,
                peer_id: None,
            }.into(),
            pts: update.pts,
            pts_count: update.pts_count,
        }.into(),
        date: update.date,
    }
}

pub fn convert_update_short_chat_message(update: tl::types::UpdateShortChatMessage) -> tl::types::UpdateShort {
    tl::types::UpdateShort {
        update: tl::types::UpdateNewMessage {
            message: tl::types::Message {
                out: update.out,
                mentioned: update.mentioned,
                media_unread: update.media_unread,
                silent: update.silent,
                post: false,
                from_scheduled: false,
                legacy: false,
                edit_hide: false,
                pinned: false,
                noforwards: false,
                invert_media: false,
                offline: false,
                video_processing_pending: false,
                paid_suggested_post_stars: false,
                paid_suggested_post_ton: false,
                id: update.id,
                from_id: Some(tl::types::PeerUser {
                    user_id: update.from_id,
                }.into()),
                from_boosts_applied: None,
                peer_id: tl::types::PeerChat {
                    chat_id: update.chat_id,
                }.into(),
                saved_peer_id: None,
                fwd_from: update.fwd_from,
                via_bot_id: update.via_bot_id,
                via_business_bot_id: None,
                reply_to: update.reply_to,
                date: update.date,
                message: update.message,
                media: None,
                reply_markup: None,
                entities: update.entities,
                views: None,
                forwards: None,
                replies: None,
                edit_date: None,
                post_author: None,
                grouped_id: None,
                reactions: None,
                restriction_reason: None,
                ttl_period: update.ttl_period,
                quick_reply_shortcut_id: None,
                effect: None,
                factcheck: None,
                report_delivery_until_date: None,
                paid_message_stars: None,
                suggested_post: None,
                schedule_repeat_period: None,
                summary_from_language: None,
                from_rank: None,
            }.into(),
            pts: update.pts,
            pts_count: update.pts_count,
        }.into(),
        date: update.date,
    }
}

fn is_common_peer(peer: &tl::enums::Peer) -> bool {
    match peer {
        tl::enums::Peer::PeerUser(_) => true,
        tl::enums::Peer::PeerChat(_) => true,
        tl::enums::Peer::PeerChannel(_) => false,
    }
}
