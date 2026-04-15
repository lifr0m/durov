use durov_mtproto::protocols::encrypted::object::Object;
use durov_tl_types::schemas::api as tl;
use std::any::Any;
use std::mem;
use tokio::sync::mpsc;

enum Sequence {
    Common,
    Channel(i64),
    Unknown,
}

pub fn redirect_updates(
    queue: &mpsc::UnboundedSender<tl::enums::Updates>,
    req: &dyn Any,
    resp: &mut Object,
) {
    if let Some(updates) = extract_updates(resp) {
        queue.send(updates).ok();
    }

    if let Some((sequence, pts, pts_count)) = extract_pts(req, resp)
        && let Some(update) = create_update(sequence, pts, pts_count)
    {
        let updates = tl::types::UpdateShort {
            update,
            date: 0,
        }.into();

        queue.send(updates).ok();
    }
}

fn extract_updates(resp: &mut Object) -> Option<tl::enums::Updates> {
    let empty = tl::types::Updates {
        updates: Vec::new(),
        users: Vec::new(),
        chats: Vec::new(),
        date: 0,
        seq: 0,
    }.into();

    if let Some(resp) = resp.downcast_mut::<tl::enums::Updates>() {
        return Some(mem::replace(resp, empty));
    }

    if let Some(resp) = resp.downcast_mut::<tl::enums::messages::InvitedUsers>() {
        let tl::enums::messages::InvitedUsers::InvitedUsers(resp) = resp;
        return Some(mem::replace(&mut resp.updates, empty));
    }

    if let Some(resp) = resp.downcast_mut::<tl::enums::payments::PaymentResult>()
        && let tl::enums::payments::PaymentResult::PaymentResult(resp) = resp
    {
        return Some(mem::replace(&mut resp.updates, empty));
    }

    None
}

fn extract_pts(req: &dyn Any, resp: &Object) -> Option<(Sequence, i32, i32)> {
    if let Some(resp) = resp.downcast_ref::<tl::enums::messages::AffectedHistory>() {
        let tl::enums::messages::AffectedHistory::AffectedHistory(resp) = resp;
        let sequence = extract_sequence(req, "messages.AffectedHistory");
        return Some((sequence, resp.pts, resp.pts_count));
    }

    if let Some(resp) = resp.downcast_ref::<tl::enums::messages::AffectedMessages>() {
        let tl::enums::messages::AffectedMessages::AffectedMessages(resp) = resp;
        let sequence = extract_sequence(req, "messages.AffectedMessages");
        return Some((sequence, resp.pts, resp.pts_count));
    }

    if let Some(resp) = resp.downcast_ref::<tl::enums::messages::AffectedFoundMessages>() {
        let tl::enums::messages::AffectedFoundMessages::AffectedFoundMessages(resp) = resp;
        let sequence = extract_sequence(req, "messages.AffectedFoundMessages");
        return Some((sequence, resp.pts, resp.pts_count));
    }

    None
}

fn extract_sequence(req: &dyn Any, resp: &str) -> Sequence {

    // messages.AffectedHistory

    if let Some(req) = req.downcast_ref::<tl::functions::messages::DeleteHistory>() {
        return sequence_from_peer(&req.peer);
    }

    if let Some(req) = req.downcast_ref::<tl::functions::messages::ReadMentions>() {
        return sequence_from_peer(&req.peer);
    }

    if let Some(req) = req.downcast_ref::<tl::functions::messages::UnpinAllMessages>() {
        return sequence_from_peer(&req.peer);
    }

    if let Some(req) = req.downcast_ref::<tl::functions::messages::ReadReactions>() {
        return sequence_from_peer(&req.peer);
    }

    if let Some(req) = req.downcast_ref::<tl::functions::messages::DeleteSavedHistory>() {
        return req.parent_peer.as_ref()
            .map(sequence_from_peer)
            .unwrap_or(Sequence::Common);
    }

    if let Some(req) = req.downcast_ref::<tl::functions::messages::DeleteTopicHistory>() {
        return sequence_from_peer(&req.peer);
    }

    if let Some(req) = req.downcast_ref::<tl::functions::channels::DeleteParticipantHistory>() {
        return sequence_from_channel(&req.channel);
    }

    // messages.AffectedMessages

    if let Some(req) = req.downcast_ref::<tl::functions::messages::ReadHistory>() {
        return sequence_from_peer(&req.peer);
    }

    if req.is::<tl::functions::messages::DeleteMessages>() {
        return Sequence::Common;
    }

    if req.is::<tl::functions::messages::ReadMessageContents>() {
        return Sequence::Common;
    }

    if let Some(req) = req.downcast_ref::<tl::functions::channels::DeleteMessages>() {
        return sequence_from_channel(&req.channel);
    }

    // messages.AffectedFoundMessages

    if req.is::<tl::functions::messages::DeletePhoneCallHistory>() {
        return Sequence::Common;
    }

    // Unknown

    log::warn!("functions which return {resp} have changed, can't detect sequence");
    Sequence::Unknown
}

fn create_update(sequence: Sequence, pts: i32, pts_count: i32) -> Option<tl::enums::Update> {
    match sequence {
        Sequence::Common => Some(
            tl::types::UpdateDeleteMessages {
                messages: Vec::new(),
                pts,
                pts_count,
            }.into()
        ),
        Sequence::Channel(channel_id) => Some(
            tl::types::UpdateDeleteChannelMessages {
                channel_id,
                messages: Vec::new(),
                pts,
                pts_count,
            }.into()
        ),
        Sequence::Unknown => None,
    }
}

fn sequence_from_peer(peer: &tl::enums::InputPeer) -> Sequence {
    match peer {
        tl::enums::InputPeer::InputPeerEmpty(_) => Sequence::Common,
        tl::enums::InputPeer::InputPeerSelf(_) => Sequence::Common,
        tl::enums::InputPeer::InputPeerChat(_) => Sequence::Common,
        tl::enums::InputPeer::InputPeerUser(_) => Sequence::Common,
        tl::enums::InputPeer::InputPeerChannel(peer) => Sequence::Channel(peer.channel_id),
        tl::enums::InputPeer::InputPeerUserFromMessage(_) => Sequence::Common,
        tl::enums::InputPeer::InputPeerChannelFromMessage(peer) => Sequence::Channel(peer.channel_id),
    }
}

fn sequence_from_channel(channel: &tl::enums::InputChannel) -> Sequence {
    match channel {
        tl::enums::InputChannel::InputChannelEmpty(_) => Sequence::Unknown,
        tl::enums::InputChannel::InputChannel(channel) => Sequence::Channel(channel.channel_id),
        tl::enums::InputChannel::InputChannelFromMessage(channel) => Sequence::Channel(channel.channel_id),
    }
}
