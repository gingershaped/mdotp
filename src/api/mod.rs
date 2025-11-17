use std::sync::Arc;

use axum::{Json, Router, extract::rejection::PathRejection, http::StatusCode, response::IntoResponse};

use crate::{AppState, presence::PresenceError};

pub mod v1;

pub struct ErrorResponse(StatusCode, mdotp_types::Error<'static>);

impl ErrorResponse {
    pub fn generic_bad_request(message: impl ToString) -> Self {
        Self(
            StatusCode::BAD_REQUEST,
            mdotp_types::Error::generic_bad_request(message)
        )
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        (self.0, Json(self.1)).into_response()
    }
}

impl From<PresenceError> for ErrorResponse {
    fn from(value: PresenceError) -> Self {
        Self(
            match value {
                PresenceError::NotTracked(..) => StatusCode::NOT_FOUND,
                PresenceError::PresenceUnavailable(..) => StatusCode::BAD_REQUEST,
                PresenceError::SdkError(..) => StatusCode::INTERNAL_SERVER_ERROR,
            },
            mdotp_types::Error {
                error_code: (&value).into(),
                message: value.to_string(),
            }
        )
    }
}

impl From<PathRejection> for ErrorResponse {
    fn from(rejection: PathRejection) -> Self {
        Self(
            rejection.status(),
            mdotp_types::Error::generic_bad_request(rejection.body_text()),
        )
    }
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().nest("/v1/", v1::routes())
}
