use std::sync::Arc;

use axum::{Json, Router, extract::rejection::PathRejection, http::StatusCode, response::IntoResponse};
use serde::Serialize;

use crate::{AppState, presence::PresenceError};

pub mod v1;

#[derive(Serialize)]
pub struct AppError {
    #[serde(rename = "error")]
    error_code: &'static str,
    message: String,
    #[serde(skip)]
    status: StatusCode,
}

impl AppError {
    const GENERIC_BAD_REQUEST_CODE: &'static str = "bad_request";

    pub fn generic_bad_request(message: impl ToString) -> Self {

        Self {
            error_code: Self::GENERIC_BAD_REQUEST_CODE,
            message: message.to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (self.status.clone(), Json(self)).into_response()
    }
}

impl From<PresenceError> for AppError {
    fn from(value: PresenceError) -> Self {
        AppError {
            error_code: (&value).into(),
            message: value.to_string(),
            status: match value {
                PresenceError::NotTracked(..) => StatusCode::NOT_FOUND,
                PresenceError::PresenceUnavailable(..) => StatusCode::BAD_REQUEST,
                PresenceError::SdkError(..) => StatusCode::INTERNAL_SERVER_ERROR,
            }
        }
    }
}

impl From<PathRejection> for AppError {
    fn from(rejection: PathRejection) -> Self {
        AppError {
            error_code: Self::GENERIC_BAD_REQUEST_CODE,
            message: rejection.body_text(),
            status: rejection.status(),
        }
    }
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().nest("/v1/", v1::routes())
}
