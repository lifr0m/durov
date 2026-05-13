pub mod updater;
mod convert;
mod action;
mod pts;
mod queue;

use crate::client::updates::action::{decide_action, Action};
use crate::client::updates::convert::*;
use crate::client::updates::pts::{extract_pts, Sequence};
use crate::client::updates::queue::Queue;
use crate::client::updates::updater::Updater;
use crate::client::Client;
use crate::sessions::encoding::PeerType;
use crate::sessions::peer::Peer;
use crate::sessions::Session;
use crate::{tl, Error};
use durov_mtproto::transports::Transport;
use std::collections::HashSet;
use std::iter;
use std::time::Duration;
use tokio::time;
use tokio::time::Instant;

const LONG_PERIOD: Duration = Duration::from_mins(15);
const GAP_LEEWAY: Duration = Duration::from_millis(500);

impl<T: Transport, S: Session> Client<T, S>
where
    T: Send + 'static,
{
    pub async fn save_updates(&self) -> Result<(), Error> {
        let updater = self.updater.try_lock()
            .expect("you should save updates at the termination");

        self.session.set_states(updater.states()).await?;

        Ok(())
    }

    pub async fn next_unauthorized_updates(&self) -> Result<Vec<tl::enums::Update>, Error> {
        let updates = self.client.read().await
            .next().await?;
        Ok(match convert_received_updates(updates) {
            Some(updates) => updates.updates,
            None => Vec::new(),
        })
    }

    pub async fn next_authorized_updates(&self) -> Result<Vec<tl::enums::Update>, Error> {
        let mut updater = self.updater.try_lock()
            .expect("you can listen for updates only from one task");

        if !updater.initialized() {
            let current = self.call(tl::functions::updates::GetState {}).await?;
            let tl::enums::updates::State::State(current) = current;

            let users = self.call(tl::functions::users::GetUsers {
                id: vec![tl::types::InputUserSelf {}.into()],
            }).await?;
            let me = match &users[..] {
                [tl::enums::User::User(user)] => user,
                _ => panic!("received invalid users"),
            };

            let states = self.session.get_states().await?;
            updater.initialize(states);

            if !self.config.catch_up || !updater.have_state(0) {
                updater.set_state(0, |state| {
                    state.pts = current.pts;
                    state.qts = current.qts;
                    state.date = current.date;
                    state.seq = current.seq;
                });
            }

            if !self.config.catch_up {
                updater.reset_channels();
            }

            updater.bot = me.bot;
        }

        {
            let updates = iter::empty()
                .chain(self.process_seq_queue(&mut updater).await?)
                .chain(self.process_pts_queue(&mut updater))
                .chain(self.process_qts_queue(&mut updater))
                .chain({
                    updater.channel_queues.keys()
                        .copied()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .flat_map(|id| self.process_channel_queue(&mut updater, id))
                })
                .collect::<Vec<_>>();

            if !updates.is_empty() {
                return Ok(updates);
            }
        }

        if updater.recovering.contains(&0) {
            let (updates, stop) = self.recover_gap(&mut updater).await?;
            if stop {
                updater.recovering.remove(&0);
            }
            return Ok(updates);
        }

        for id in updater.recovering.iter()
            .copied()
            .collect::<Vec<_>>()
        {
            match self.session.get_peer_by_id(id, PeerType::Channel).await? {
                Some(peer) => {
                    let (updates, stop) = self.recover_channel_gap(&mut updater, peer).await?;
                    if stop {
                        updater.recovering.remove(&id);
                    }
                    return Ok(updates);
                }
                None => {
                    tracing::error!(id, "failed to recover channel gap, don't have peer information");
                    updater.recovering.remove(&id);
                    updater.channel_queues.remove(&id);
                }
            }
        }

        let timeout = iter::empty()
            .chain([updater.seq_queue.gap_since])
            .chain([updater.pts_queue.gap_since])
            .chain([updater.qts_queue.gap_since])
            .chain(
                updater.channel_queues.values()
                    .map(|queue| queue.gap_since)
            )
            .flatten()
            .min()
            .map(|since| GAP_LEEWAY - since.elapsed())
            .unwrap_or(LONG_PERIOD);

        match time::timeout(timeout, self.client.read().await.next()).await {
            Ok(updates) => {
                self.process_received_updates(&mut updater, updates?);
            }
            Err(_) => if timeout == LONG_PERIOD {
                updater.recovering.insert(0);
            }
        }

        Ok(Vec::new())
    }

    fn process_received_updates(&self, updater: &mut Updater, updates: tl::enums::Updates) {
        match convert_received_updates(updates) {
            Some(updates) => {
                let seq_start = updates.seq_start;
                updater.seq_queue.put(updates, seq_start, 0);
            }
            None => {
                updater.recovering.insert(0);
            }
        }
    }

    async fn process_seq_queue(&self, updater: &mut Updater)
        -> Result<Vec<tl::enums::Update>, Error>
    {
        let mut result = Vec::new();

        while let Some((seq_start, _)) = updater.seq_queue.peek() {
            let current = updater.get_state(0);

            match decide_action(current.seq, 1, seq_start) {
                Action::Apply => {
                    if seq_start != 0 {
                        updater.seq_queue.gap_since = None;
                    }

                    let updates = updater.seq_queue.take()
                        .remove(0);

                    self.apply_peers(&updates.chats, &updates.users).await?;

                    updater.set_state(0, |state| {
                        if updates.date != 0 {
                            state.date = updates.date;
                        }
                        if updates.seq != 0 {
                            state.seq = updates.seq;
                        }
                    });

                    result.extend(self.process_updates(updater, updates.updates));
                }
                Action::Ignore => {
                    updater.seq_queue.take();
                }
                Action::FillGap => {
                    self.process_gap(&mut updater.recovering, 0, &mut updater.seq_queue);
                    break;
                }
            }
        }

        Ok(result)
    }

    fn process_updates(&self, updater: &mut Updater, updates: Vec<tl::enums::Update>)
        -> Vec<tl::enums::Update>
    {
        let mut result = Vec::new();

        for update in updates {
            match update {
                tl::enums::Update::UpdateChannelTooLong(update) => {
                    self.process_update_channel_too_long(updater, update);
                }
                _ => self.process_regular_update(updater, update, &mut result),
            }
        }

        result
    }

    fn process_update_channel_too_long(
        &self,
        updater: &mut Updater,
        update: tl::types::UpdateChannelTooLong,
    ) {
        updater.recovering.insert(update.channel_id);

        if !updater.have_state(update.channel_id) {
            updater.set_state(update.channel_id, |state| {
                state.pts = update.pts.unwrap();
            });
        }
    }

    fn process_regular_update(
        &self,
        updater: &mut Updater,
        update: tl::enums::Update,
        result: &mut Vec<tl::enums::Update>,
    ) {
        match extract_pts(&update) {
            Some((Sequence::Common, pts, count)) if pts != 0 => {
                updater.pts_queue.put(update, pts, count);
            }
            Some((Sequence::Secondary, qts, count)) if qts != 0 => {
                updater.qts_queue.put(update, qts, count);
            }
            Some((Sequence::Channel(id), pts, count)) if pts != 0 => {
                updater.channel_queues.entry(id)
                    .or_default()
                    .put(update, pts, count);
            }
            _ => result.push(update),
        }
    }

    fn process_pts_queue(&self, updater: &mut Updater) -> Vec<tl::enums::Update> {
        let mut result = Vec::new();

        while let Some((pts, count)) = updater.pts_queue.peek() {
            let current = updater.get_state(0);

            match decide_action(current.pts, count, pts) {
                Action::Apply => {
                    updater.pts_queue.gap_since = None;

                    updater.set_state(0, |state| {
                        state.pts = pts;
                    });
                    result.extend(updater.pts_queue.take());
                }
                Action::Ignore => {
                    updater.pts_queue.take();
                }
                Action::FillGap => {
                    self.process_gap(&mut updater.recovering, 0, &mut updater.pts_queue);
                    break;
                }
            }
        }

        result
    }

    fn process_qts_queue(&self, updater: &mut Updater) -> Vec<tl::enums::Update> {
        let mut result = Vec::new();

        while let Some((qts, count)) = updater.qts_queue.peek() {
            let current = updater.get_state(0);

            match decide_action(current.qts, count, qts) {
                Action::Apply => {
                    updater.qts_queue.gap_since = None;

                    updater.set_state(0, |state| {
                        state.qts = qts;
                    });
                    result.extend(updater.qts_queue.take());
                }
                Action::Ignore => {
                    updater.qts_queue.take();
                }
                Action::FillGap => {
                    self.process_gap(&mut updater.recovering, 0, &mut updater.qts_queue);
                    break;
                }
            }
        }

        result
    }

    fn process_channel_queue(&self, updater: &mut Updater, id: i64) -> Vec<tl::enums::Update> {
        let mut result = Vec::new();

        fn queue(updater: &mut Updater, id: i64) -> &mut Queue<tl::enums::Update> {
            updater.channel_queues.get_mut(&id).unwrap()
        }

        if !updater.have_state(id) && let Some((pts, _)) = queue(updater, id).peek() {
            updater.set_state(id, |state| {
                state.pts = pts;
            });
            result.extend(queue(updater, id).take());
        }

        while let Some((pts, count)) = queue(updater, id).peek() {
            let current = updater.get_state(id);

            match decide_action(current.pts, count, pts) {
                Action::Apply => {
                    queue(updater, id).gap_since = None;

                    updater.set_state(id, |state| {
                        state.pts = pts;
                    });
                    result.extend(queue(updater, id).take());
                }
                Action::Ignore => {
                    queue(updater, id).take();
                }
                Action::FillGap => {
                    let queue = updater.channel_queues.get_mut(&id).unwrap();
                    self.process_gap(&mut updater.recovering, id, queue);
                    break;
                }
            }
        }

        result
    }

    fn process_gap<I>(&self, recovering: &mut HashSet<i64>, id: i64, queue: &mut Queue<I>) {
        if let Some(since) = queue.gap_since {
            if since.elapsed() > GAP_LEEWAY {
                recovering.insert(id);
                queue.gap_since = None;
            }
        } else if !recovering.contains(&id) {
            queue.gap_since = Some(Instant::now());
        }
    }

    async fn recover_gap(&self, updater: &mut Updater)
        -> Result<(Vec<tl::enums::Update>, bool), Error>
    {
        let current = updater.get_state(0);
        let diff = self.call(tl::functions::updates::GetDifference {
            pts: current.pts,
            pts_limit: None,
            pts_total_limit: None,
            date: current.date,
            qts: current.qts,
            qts_limit: None,
        }).await?;

        Ok(match diff {
            tl::enums::updates::Difference::DifferenceEmpty(diff) => {
                updater.set_state(0, |state| {
                    state.date = diff.date;
                    state.seq = diff.seq;
                });

                (Vec::new(), true)
            }
            tl::enums::updates::Difference::Difference(diff) => {
                let mut updates = diff.other_updates;

                for message in diff.new_messages {
                    updates.push(convert_new_message(message));
                }
                for message in diff.new_encrypted_messages {
                    updates.push(convert_new_encrypted_message(message));
                }

                self.apply_peers(&diff.chats, &diff.users).await?;

                let tl::enums::updates::State::State(current) = diff.state;
                updater.set_state(0, |state| {
                    state.pts = current.pts;
                    state.qts = current.qts;
                    state.date = current.date;
                    state.seq = current.seq;
                });

                (self.process_updates(updater, updates), true)
            }
            tl::enums::updates::Difference::DifferenceSlice(diff) => {
                let mut updates = diff.other_updates;

                for message in diff.new_messages {
                    updates.push(convert_new_message(message));
                }
                for message in diff.new_encrypted_messages {
                    updates.push(convert_new_encrypted_message(message));
                }

                self.apply_peers(&diff.chats, &diff.users).await?;

                let tl::enums::updates::State::State(current) = diff.intermediate_state;
                updater.set_state(0, |state| {
                    state.pts = current.pts;
                    state.qts = current.qts;
                    state.date = current.date;
                    state.seq = current.seq;
                });

                (self.process_updates(updater, updates), false)
            }
            tl::enums::updates::Difference::DifferenceTooLong(diff) => {
                updater.set_state(0, |state| {
                    state.pts = diff.pts;
                });

                (Vec::new(), false)
            }
        })
    }

    async fn recover_channel_gap(&self, updater: &mut Updater, peer: Peer)
        -> Result<(Vec<tl::enums::Update>, bool), Error>
    {
        let current = updater.get_state(peer.id);
        let diff = self.call(tl::functions::updates::GetChannelDifference {
            force: false,
            channel: peer.to_input_channel(),
            filter: tl::types::ChannelMessagesFilterEmpty {}.into(),
            pts: current.pts,
            limit: if updater.bot { 100_000 } else { 100 },
        }).await?;

        Ok(match diff {
            tl::enums::updates::ChannelDifference::ChannelDifferenceEmpty(diff) => {
                updater.set_state(peer.id, |state| {
                    state.pts = diff.pts;
                });

                (Vec::new(), true)
            }
            tl::enums::updates::ChannelDifference::ChannelDifference(diff) => {
                let mut updates = diff.other_updates;

                for message in diff.new_messages {
                    updates.push(convert_new_message(message));
                }

                self.apply_peers(&diff.chats, &diff.users).await?;

                updater.set_state(peer.id, |state| {
                    state.pts = diff.pts;
                });

                (self.process_updates(updater, updates), diff.final_)
            }
            tl::enums::updates::ChannelDifference::ChannelDifferenceTooLong(diff) => {
                self.apply_peers(&diff.chats, &diff.users).await?;

                match diff.dialog {
                    tl::enums::Dialog::Dialog(dialog) => {
                        updater.set_state(peer.id, |state| {
                            state.pts = dialog.pts.unwrap();
                        });
                    }
                    tl::enums::Dialog::DialogFolder(_) => panic!("dialog is actually a folder"),
                }

                (Vec::new(), true)
            }
        })
    }
}

fn convert_received_updates(updates: tl::enums::Updates) -> Option<tl::types::UpdatesCombined> {
    match updates {
        tl::enums::Updates::Updates(updates) => convert_received_updates(
            convert_updates(updates).into()
        ),
        tl::enums::Updates::UpdatesCombined(updates) => Some(updates),
        tl::enums::Updates::UpdateShort(updates) => convert_received_updates(
            convert_update_short(updates).into()
        ),
        tl::enums::Updates::UpdateShortMessage(updates) => convert_received_updates(
            convert_update_short_message(updates).into()
        ),
        tl::enums::Updates::UpdateShortSentMessage(updates) => convert_received_updates(
            convert_update_short_sent_message(updates).into()
        ),
        tl::enums::Updates::UpdateShortChatMessage(updates) => convert_received_updates(
            convert_update_short_chat_message(updates).into()
        ),
        tl::enums::Updates::UpdatesTooLong(_) => None,
    }
}
