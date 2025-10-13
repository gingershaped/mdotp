use std::sync::Arc;

use axum::{
    extract::{ws::Message, Path, State, WebSocketUpgrade}, response::Response, routing::get, Json, Router
};
use matrix_sdk::ruma::OwnedUserId;

use crate::{
    AppState,
    presence::{Presence, PresenceError},
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/user/{user}", get(user))
        .route("/user/{user}/ws", get(user_ws))
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

async fn user_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<OwnedUserId>,
) -> Result<Response, Json<PresenceError>> {
    let mut rx = state.presences.presence_for(&user_id).await?;

    Ok(ws.on_upgrade(|mut ws| async move {
        loop {
            let message = {
                let presence = rx.borrow_and_update();
                Message::text(serde_json::to_string(&*presence).unwrap())
            };
            if ws.send(message).await.is_err() {
                break;
            }
            if rx.changed().await.is_err() {
                break;
            }
        }
    }))
}