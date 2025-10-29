use std::{collections::HashMap, sync::Arc};

use matrix_sdk::{
    event_handler::EventHandlerDropGuard, ruma::{
        api::client::presence::get_presence, events::{
            direct::DirectUserIdentifier, presence::PresenceEvent, room::{
                member::{
                    MembershipChange, MembershipState, OriginalSyncRoomMemberEvent,
                },
                message::RoomMessageEventContent,
            }
        }, presence::PresenceState, OwnedMxcUri, OwnedUserId, UserId
    }, Client, Room
};
use ruma_common::serde::Raw;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{RwLock, watch};
use tracing::{debug, error, info, warn};

#[derive(Error, Debug, Serialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum PresenceError {
    #[error("mdotp is not tracking this user")]
    NotTracked(#[serde(skip)] OwnedUserId),

    #[error("presence information is unavailable for this user")]
    PresenceUnavailable(#[serde(skip)] OwnedUserId),

    #[error("internal Matrix error")]
    InternalError(#[serde(skip)] matrix_sdk::Error),
}

impl From<matrix_sdk::Error> for PresenceError {
    fn from(value: matrix_sdk::Error) -> Self {
        error!(?value, "matrix sdk error");
        PresenceError::InternalError(value)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Presence {
    /// The current avatar URL for this user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<OwnedMxcUri>,

    /// The current display name for this user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub displayname: Option<String>,

    /// Whether or not the user is currently active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currently_active: Option<bool>,

    /// The last time since this user performed some action, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_active_ago: Option<u128>,

    /// The presence state for this user.
    pub presence: PresenceState,

    /// An optional description to accompany the presence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_msg: Option<String>,
}

pub struct Presences {
    room: Room,
    presence_senders: Arc<RwLock<HashMap<OwnedUserId, watch::Sender<Presence>>>>,
    _handlers: [EventHandlerDropGuard; 2],
}

impl Presences {
    pub fn new(room: Room) -> Self {
        let client = room.client();
        let presence_senders = Arc::new(RwLock::new(
            HashMap::<OwnedUserId, watch::Sender<Presence>>::new(),
        ));

        let handlers = {
            [
                {
                    let presence_senders = presence_senders.clone();
                    client.event_handler_drop_guard(room.add_event_handler(
                        |event: OriginalSyncRoomMemberEvent, client: Client| async move {
                            let user_id = &event.state_key;
                            match event.content.membership {
                                MembershipState::Leave => {
                                    debug!(?user_id, "user left the room");
                                    let mut presence_senders = presence_senders.write().await;
                                    let _ = presence_senders.remove(user_id);
                                }
                                MembershipState::Join => {
                                    let presence_senders = presence_senders.read().await;

                                    if matches!(
                                        event.membership_change(),
                                        MembershipChange::Joined
                                            | MembershipChange::InvitationAccepted
                                    ) {
                                        debug!(?user_id, "user joined the room");
                                        if let Err(err) =
                                            Self::check_presence_available(&client, user_id).await
                                        {
                                            warn!(
                                                ?user_id,
                                                ?err,
                                                "error checking presence availability"
                                            )
                                        }
                                    }

                                    if let Some(tx) = presence_senders.get(&event.sender) {
                                        debug!(?user_id, ?event.content, "new user profile");
                                        tx.send_modify(|presence| {
                                            presence.avatar_url = event.content.avatar_url.clone();
                                            presence.displayname =
                                                event.content.displayname.clone();
                                        });
                                    }
                                }
                                _ => {}
                            }
                        },
                    ))
                },
                {
                    let presence_senders = presence_senders.clone();
                    client.event_handler_drop_guard(client.add_event_handler(
                        |event: PresenceEvent| async move {
                            let presence_senders = presence_senders.read().await;
                            if let Some(tx) = presence_senders.get(&event.sender) {
                                debug!(?event.sender, ?event.content, "new presence");
                                tx.send_modify(|presence| {
                                    presence.currently_active = event.content.currently_active;
                                    presence.last_active_ago =
                                        event.content.last_active_ago.map(|ts| ts.into());
                                    presence.presence = event.content.presence;
                                    presence.status_msg = event.content.status_msg;
                                });
                            }
                        },
                    ))
                },
            ]
        };

        Self {
            room,
            presence_senders,
            _handlers: handlers,
        }
    }

    async fn check_presence_available(
        client: &Client,
        user_id: &UserId,
    ) -> Result<(), matrix_sdk::Error> {
        const NOTIFICATION_FLAG: &'static str =
            "computer.gingershaped.mdotp.presence_unavailable_notified";
        const PRESENCE_UNAVAILABLE_NOTICE: &'static str = concat!(
            "<b>NOTICE:</b> Your homeserver does not currently publish presence information, so you will not be able ",
            "to access your presence through mdotp. To resolve this, contact your homeserver's administrators."
        );

        let response = client
            .send(get_presence::v3::Request::new(user_id.to_owned()))
            .await;

        if response.is_err() {
            let dm_room = match Self::get_dm_room(&client, user_id) {
                Some(room) => room,
                None => client.create_dm(user_id).await?,
            };

            if dm_room
                .account_data(NOTIFICATION_FLAG.into())
                .await?
                .is_none()
            {
                dm_room
                    .send(RoomMessageEventContent::notice_html(
                        PRESENCE_UNAVAILABLE_NOTICE,
                        PRESENCE_UNAVAILABLE_NOTICE,
                    ))
                    .await?;
                dm_room
                    .set_account_data_raw(
                        NOTIFICATION_FLAG.into(),
                        Raw::new(&true).unwrap().cast_unchecked(),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    // copied from matrix-rust-sdk
    fn get_dm_room(client: &Client, user_id: &UserId) -> Option<Room> {
        let rooms = client.joined_rooms();

        // Find the room we share with the `user_id` and only with `user_id`
        let room = rooms.into_iter().find(|r| {
            let targets = r.direct_targets();
            targets.len() == 1 && targets.contains(<&DirectUserIdentifier>::from(user_id))
        });

        room
    }

    pub async fn presence_for(
        &self,
        user_id: &UserId,
    ) -> Result<watch::Receiver<Presence>, PresenceError> {
        let presence_senders = self.presence_senders.read().await;

        if let Some(tx) = presence_senders.get(user_id) {
            Ok(tx.subscribe())
        } else {
            drop(presence_senders);

            let initial_presence = self.current_presence(user_id).await?;
            info!(?user_id, "adding new presence channel");
            debug!(?user_id, ?initial_presence, "initial presence");
            let (tx, rx) = watch::channel(initial_presence);

            self.presence_senders
                .write()
                .await
                .insert(user_id.to_owned(), tx);

            Ok(rx)
        }
    }

    async fn current_presence(&self, user_id: &UserId) -> Result<Presence, PresenceError> {
        let Some(member) = self
            .room
            .get_member(user_id)
            .await?
            .filter(|member| *member.membership() == MembershipState::Join)
        else {
            return Err(PresenceError::NotTracked(user_id.to_owned()));
        };

        let response = self
            .room
            .client()
            .send(get_presence::v3::Request::new(user_id.to_owned()))
            .await
            .map_err(|_| PresenceError::PresenceUnavailable(user_id.to_owned()))?;

        Ok(Presence {
            avatar_url: member.avatar_url().map(ToOwned::to_owned),
            displayname: member.display_name().map(ToOwned::to_owned),
            currently_active: response.currently_active,
            last_active_ago: response
                .last_active_ago
                .map(|duration| duration.as_millis()),
            presence: response.presence,
            status_msg: response.status_msg,
        })
    }
}
