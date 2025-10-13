use axum::{http::StatusCode, response::IntoResponse, Json};

use crate::presence::PresenceError;

pub mod v1;

impl IntoResponse for PresenceError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match self {
            Self::InternalError(..) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        };

        (status_code, Json(self)).into_response()
    }
}