use std::sync::Arc;

use axum::{
    Json, Router, extract::{Path, State, WebSocketUpgrade, rejection::PathRejection, ws::Message}, response::Response, routing::get
};
use matrix_sdk::ruma::OwnedUserId;

use crate::{
    AppState, api::AppError, presence::Presence
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/user/{user}", get(user))
        .route("/user/{user}/ws", get(user_ws))
}

async fn user(
    State(state): State<Arc<AppState>>,
    user_id: Result<Path<OwnedUserId>, PathRejection>,
) -> Result<Json<Presence>, AppError> {
    Ok(Json(
        state
            .presences
            .presence_for(&user_id?)
            .await?
            .borrow()
            .clone(),
    ))
}

async fn user_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    user_id: Result<Path<OwnedUserId>, PathRejection>,
) -> Result<Response, AppError> {
    let mut rx = state.presences.presence_for(&user_id?).await?;

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
        
        let _ = ws.send(Message::Close(None)).await;
    }))
}