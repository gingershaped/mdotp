use std::sync::Arc;

use axum::{
    extract::{Path, State}, routing::get, Json, Router
};
use matrix_sdk::ruma::OwnedUserId;

use crate::{
    AppState,
    presence::{Presence, PresenceError},
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/user/{user}", get(user))
}

async fn user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<OwnedUserId>,
) -> Result<Json<Presence>, Json<PresenceError>> {
    Ok(Json(
        state
            .presences
            .presence_for(&user_id)
            .await?
            .borrow()
            .clone(),
    ))
}
