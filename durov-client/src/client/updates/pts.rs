use crate::tl;

pub enum Sequence {
    Common,
    Secondary,
    Channel(i64),
}

pub fn extract_pts(update: &tl::enums::Update) -> Option<(Sequence, i32, i32)> {
    match update {
        tl::enums::Update::UpdateNewMessage(update) => {
            Some((Sequence::Common, update.pts, update.pts_count))
        }
        tl::enums::Update::UpdateDeleteMessages(update) => {
            Some((Sequence::Common, update.pts, update.pts_count))
        }
        tl::enums::Update::UpdateReadHistoryInbox(update) => {
            Some((Sequence::Common, update.pts, update.pts_count))
        }
        tl::enums::Update::UpdateReadHistoryOutbox(update) => {
            Some((Sequence::Common, update.pts, update.pts_count))
        }
        tl::enums::Update::UpdateWebPage(update) => {
            Some((Sequence::Common, update.pts, update.pts_count))
        }
        tl::enums::Update::UpdateReadMessagesContents(update) => {
            Some((Sequence::Common, update.pts, update.pts_count))
        }
        tl::enums::Update::UpdateNewChannelMessage(update) => {
            Some((Sequence::Channel(extract_channel_id(&update.message)), update.pts, update.pts_count))
        }
        tl::enums::Update::UpdateReadChannelInbox(update) => {
            Some((Sequence::Channel(update.channel_id), update.pts, 0))
        }
        tl::enums::Update::UpdateDeleteChannelMessages(update) => {
            Some((Sequence::Channel(update.channel_id), update.pts, update.pts_count))
        }
        tl::enums::Update::UpdateEditChannelMessage(update) => {
            Some((Sequence::Channel(extract_channel_id(&update.message)), update.pts, update.pts_count))
        }
        tl::enums::Update::UpdateEditMessage(update) => {
            Some((Sequence::Common, update.pts, update.pts_count))
        }
        tl::enums::Update::UpdateChannelWebPage(update) => {
            Some((Sequence::Channel(update.channel_id), update.pts, update.pts_count))
        }
        tl::enums::Update::UpdateFolderPeers(update) => {
            Some((Sequence::Common, update.pts, update.pts_count))
        }
        tl::enums::Update::UpdatePinnedMessages(update) => {
            Some((Sequence::Common, update.pts, update.pts_count))
        }
        tl::enums::Update::UpdatePinnedChannelMessages(update) => {
            Some((Sequence::Channel(update.channel_id), update.pts, update.pts_count))
        }
        tl::enums::Update::UpdateNewEncryptedMessage(update) => {
            Some((Sequence::Secondary, update.qts, 1))
        }
        tl::enums::Update::UpdateMessagePollVote(update) => {
            Some((Sequence::Secondary, update.qts, 1))
        }
        tl::enums::Update::UpdateChatParticipant(update) => {
            Some((Sequence::Secondary, update.qts, 1))
        }
        tl::enums::Update::UpdateChannelParticipant(update) => {
            Some((Sequence::Secondary, update.qts, 1))
        }
        tl::enums::Update::UpdateBotStopped(update) => {
            Some((Sequence::Secondary, update.qts, 1))
        }
        tl::enums::Update::UpdateBotChatInviteRequester(update) => {
            Some((Sequence::Secondary, update.qts, 1))
        }
        tl::enums::Update::UpdateBotChatBoost(update) => {
            Some((Sequence::Secondary, update.qts, 1))
        }
        tl::enums::Update::UpdateBotMessageReaction(update) => {
            Some((Sequence::Secondary, update.qts, 1))
        }
        tl::enums::Update::UpdateBotMessageReactions(update) => {
            Some((Sequence::Secondary, update.qts, 1))
        }
        tl::enums::Update::UpdateBotBusinessConnect(update) => {
            Some((Sequence::Secondary, update.qts, 1))
        }
        tl::enums::Update::UpdateBotNewBusinessMessage(update) => {
            Some((Sequence::Secondary, update.qts, 1))
        }
        tl::enums::Update::UpdateBotEditBusinessMessage(update) => {
            Some((Sequence::Secondary, update.qts, 1))
        }
        tl::enums::Update::UpdateBotDeleteBusinessMessage(update) => {
            Some((Sequence::Secondary, update.qts, update.messages.len() as i32))
        }
        tl::enums::Update::UpdateBotPurchasedPaidMedia(update) => {
            Some((Sequence::Secondary, update.qts, 1))
        }
        tl::enums::Update::UpdateChannelTooLong(_) => None,
        tl::enums::Update::UpdateMessageId(_) => None,
        tl::enums::Update::UpdateUserTyping(_) => None,
        tl::enums::Update::UpdateChatUserTyping(_) => None,
        tl::enums::Update::UpdateChatParticipants(_) => None,
        tl::enums::Update::UpdateUserStatus(_) => None,
        tl::enums::Update::UpdateUserName(_) => None,
        tl::enums::Update::UpdateNewAuthorization(_) => None,
        tl::enums::Update::UpdateEncryptedChatTyping(_) => None,
        tl::enums::Update::UpdateEncryption(_) => None,
        tl::enums::Update::UpdateEncryptedMessagesRead(_) => None,
        tl::enums::Update::UpdateChatParticipantAdd(_) => None,
        tl::enums::Update::UpdateChatParticipantDelete(_) => None,
        tl::enums::Update::UpdateDcOptions(_) => None,
        tl::enums::Update::UpdateNotifySettings(_) => None,
        tl::enums::Update::UpdateServiceNotification(_) => None,
        tl::enums::Update::UpdatePrivacy(_) => None,
        tl::enums::Update::UpdateUserPhone(_) => None,
        tl::enums::Update::UpdateChannel(_) => None,
        tl::enums::Update::UpdateChannelMessageViews(_) => None,
        tl::enums::Update::UpdateChatParticipantAdmin(_) => None,
        tl::enums::Update::UpdateNewStickerSet(_) => None,
        tl::enums::Update::UpdateStickerSetsOrder(_) => None,
        tl::enums::Update::UpdateStickerSets(_) => None,
        tl::enums::Update::UpdateSavedGifs(_) => None,
        tl::enums::Update::UpdateBotInlineQuery(_) => None,
        tl::enums::Update::UpdateBotInlineSend(_) => None,
        tl::enums::Update::UpdateBotCallbackQuery(_) => None,
        tl::enums::Update::UpdateInlineBotCallbackQuery(_) => None,
        tl::enums::Update::UpdateReadChannelOutbox(_) => None,
        tl::enums::Update::UpdateDraftMessage(_) => None,
        tl::enums::Update::UpdateReadFeaturedStickers(_) => None,
        tl::enums::Update::UpdateRecentStickers(_) => None,
        tl::enums::Update::UpdateConfig(_) => None,
        tl::enums::Update::UpdatePtsChanged(_) => None,
        tl::enums::Update::UpdateDialogPinned(_) => None,
        tl::enums::Update::UpdatePinnedDialogs(_) => None,
        tl::enums::Update::UpdateBotWebhookJson(_) => None,
        tl::enums::Update::UpdateBotWebhookJsonQuery(_) => None,
        tl::enums::Update::UpdateBotShippingQuery(_) => None,
        tl::enums::Update::UpdateBotPrecheckoutQuery(_) => None,
        tl::enums::Update::UpdatePhoneCall(_) => None,
        tl::enums::Update::UpdateLangPackTooLong(_) => None,
        tl::enums::Update::UpdateLangPack(_) => None,
        tl::enums::Update::UpdateFavedStickers(_) => None,
        tl::enums::Update::UpdateChannelReadMessagesContents(_) => None,
        tl::enums::Update::UpdateContactsReset(_) => None,
        tl::enums::Update::UpdateChannelAvailableMessages(_) => None,
        tl::enums::Update::UpdateDialogUnreadMark(_) => None,
        tl::enums::Update::UpdateMessagePoll(_) => None,
        tl::enums::Update::UpdateChatDefaultBannedRights(_) => None,
        tl::enums::Update::UpdatePeerSettings(_) => None,
        tl::enums::Update::UpdatePeerLocated(_) => None,
        tl::enums::Update::UpdateNewScheduledMessage(_) => None,
        tl::enums::Update::UpdateDeleteScheduledMessages(_) => None,
        tl::enums::Update::UpdateTheme(_) => None,
        tl::enums::Update::UpdateGeoLiveViewed(_) => None,
        tl::enums::Update::UpdateLoginToken(_) => None,
        tl::enums::Update::UpdateDialogFilter(_) => None,
        tl::enums::Update::UpdateDialogFilterOrder(_) => None,
        tl::enums::Update::UpdateDialogFilters(_) => None,
        tl::enums::Update::UpdatePhoneCallSignalingData(_) => None,
        tl::enums::Update::UpdateChannelMessageForwards(_) => None,
        tl::enums::Update::UpdateReadChannelDiscussionInbox(_) => None,
        tl::enums::Update::UpdateReadChannelDiscussionOutbox(_) => None,
        tl::enums::Update::UpdatePeerBlocked(_) => None,
        tl::enums::Update::UpdateChannelUserTyping(_) => None,
        tl::enums::Update::UpdateChat(_) => None,
        tl::enums::Update::UpdateGroupCallParticipants(_) => None,
        tl::enums::Update::UpdateGroupCall(_) => None,
        tl::enums::Update::UpdatePeerHistoryTtl(_) => None,
        tl::enums::Update::UpdateGroupCallConnection(_) => None,
        tl::enums::Update::UpdateBotCommands(_) => None,
        tl::enums::Update::UpdatePendingJoinRequests(_) => None,
        tl::enums::Update::UpdateMessageReactions(_) => None,
        tl::enums::Update::UpdateAttachMenuBots(_) => None,
        tl::enums::Update::UpdateWebViewResultSent(_) => None,
        tl::enums::Update::UpdateBotMenuButton(_) => None,
        tl::enums::Update::UpdateSavedRingtones(_) => None,
        tl::enums::Update::UpdateTranscribedAudio(_) => None,
        tl::enums::Update::UpdateReadFeaturedEmojiStickers(_) => None,
        tl::enums::Update::UpdateUserEmojiStatus(_) => None,
        tl::enums::Update::UpdateRecentEmojiStatuses(_) => None,
        tl::enums::Update::UpdateRecentReactions(_) => None,
        tl::enums::Update::UpdateMoveStickerSetToTop(_) => None,
        tl::enums::Update::UpdateMessageExtendedMedia(_) => None,
        tl::enums::Update::UpdateUser(_) => None,
        tl::enums::Update::UpdateAutoSaveSettings(_) => None,
        tl::enums::Update::UpdateStory(_) => None,
        tl::enums::Update::UpdateReadStories(_) => None,
        tl::enums::Update::UpdateStoryId(_) => None,
        tl::enums::Update::UpdateStoriesStealthMode(_) => None,
        tl::enums::Update::UpdateSentStoryReaction(_) => None,
        tl::enums::Update::UpdateChannelViewForumAsMessages(_) => None,
        tl::enums::Update::UpdatePeerWallpaper(_) => None,
        tl::enums::Update::UpdateSavedDialogPinned(_) => None,
        tl::enums::Update::UpdatePinnedSavedDialogs(_) => None,
        tl::enums::Update::UpdateSavedReactionTags(_) => None,
        tl::enums::Update::UpdateSmsJob(_) => None,
        tl::enums::Update::UpdateQuickReplies(_) => None,
        tl::enums::Update::UpdateNewQuickReply(_) => None,
        tl::enums::Update::UpdateDeleteQuickReply(_) => None,
        tl::enums::Update::UpdateQuickReplyMessage(_) => None,
        tl::enums::Update::UpdateDeleteQuickReplyMessages(_) => None,
        tl::enums::Update::UpdateNewStoryReaction(_) => None,
        tl::enums::Update::UpdateStarsBalance(_) => None,
        tl::enums::Update::UpdateBusinessBotCallbackQuery(_) => None,
        tl::enums::Update::UpdateStarsRevenueStatus(_) => None,
        tl::enums::Update::UpdatePaidReactionPrivacy(_) => None,
        tl::enums::Update::UpdateSentPhoneCode(_) => None,
        tl::enums::Update::UpdateGroupCallChainBlocks(_) => None,
        tl::enums::Update::UpdateReadMonoForumInbox(_) => None,
        tl::enums::Update::UpdateReadMonoForumOutbox(_) => None,
        tl::enums::Update::UpdateMonoForumNoPaidException(_) => None,
        tl::enums::Update::UpdateGroupCallMessage(_) => None,
        tl::enums::Update::UpdateGroupCallEncryptedMessage(_) => None,
        tl::enums::Update::UpdatePinnedForumTopic(_) => None,
        tl::enums::Update::UpdatePinnedForumTopics(_) => None,
        tl::enums::Update::UpdateDeleteGroupCallMessages(_) => None,
        tl::enums::Update::UpdateStarGiftAuctionState(_) => None,
        tl::enums::Update::UpdateStarGiftAuctionUserState(_) => None,
        tl::enums::Update::UpdateEmojiGameInfo(_) => None,
        tl::enums::Update::UpdateStarGiftCraftFail(_) => None,
        tl::enums::Update::UpdateChatParticipantRank(_) => None,
    }
}

fn extract_channel_id(message: &tl::enums::Message) -> i64 {
    match message {
        tl::enums::Message::MessageEmpty(message) => match &message.peer_id {
            Some(peer) => match peer {
                tl::enums::Peer::PeerChannel(peer) => peer.channel_id,
                _ => panic!("message is not from channel"),
            }
            None => panic!("message doesn't have peer information"),
        }
        tl::enums::Message::Message(message) => match &message.peer_id {
            tl::enums::Peer::PeerChannel(peer) => peer.channel_id,
            _ => panic!("message is not from channel"),
        }
        tl::enums::Message::MessageService(message) => match &message.peer_id {
            tl::enums::Peer::PeerChannel(peer) => peer.channel_id,
            _ => panic!("message is not from channel"),
        }
    }
}
