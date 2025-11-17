use ruma_common::{OwnedMxcUri, presence::PresenceState};
use serde::{Deserialize, Serialize};

/// A response to /api/v1/users/<mxid>/.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum PresenceResponse<'code> {
    Ok(Presence),
    #[serde(borrow)]
    Error(Error<'code>),
}

#[derive(Serialize, Deserialize)]
pub struct Error<'code> {
    #[serde(rename = "error")]
    pub error_code: &'code str,
    pub message: String,
}

impl<'code> Error<'code> {
    const GENERIC_BAD_REQUEST_CODE: &'static str = "bad_request";

    pub fn generic_bad_request(message: impl ToString) -> Self {
        Self {
            error_code: Self::GENERIC_BAD_REQUEST_CODE,
            message: message.to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
