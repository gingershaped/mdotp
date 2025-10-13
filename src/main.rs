use std::sync::Arc;

use anyhow::Context;
use axum::Router;
use matrix_sdk::{Client, config::SyncSettings, ruma::OwnedRoomId};
use mdotp::{AppState, api, presence::Presences};
use serde::Deserialize;
use tokio::task::JoinHandle;
use tracing::info;

#[derive(Deserialize)]
struct Environ {
    host: String,
    homeserver: String,
    username: String,
    password: String,
    main_room: OwnedRoomId,
}

fn load_environ() -> anyhow::Result<Environ> {
    let _ = dotenvy::dotenv();
    envy::from_env().with_context(|| "Error loading configuration")
}

async fn flatten<T>(handle: JoinHandle<Result<T, anyhow::Error>>) -> Result<T, anyhow::Error> {
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(anyhow::Error::new(err).context("task exited abnormally")),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let environ = load_environ()?;
    tracing_subscriber::fmt::init();

    let client = Client::builder()
        .homeserver_url(environ.homeserver)
        .build()
        .await?;

    client
        .matrix_auth()
        .login_username(environ.username, &environ.password)
        .initial_device_display_name("mdotp")
        .await?;

    info!("login success");

    let initial_sync_token = client.sync_once(SyncSettings::default()).await?.next_batch;

    let state = AppState {
        presences: Presences::new(
            client
                .get_room(&environ.main_room)
                .with_context(|| "Main room is unavailable")?,
        ),
    };

    let app: Router<()> = Router::new()
        .nest("/v1/", api::v1::routes())
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind(environ.host.clone()).await?;

    let app_task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .map_err(anyhow::Error::from)
    });

    let sync_task = tokio::spawn(async move {
        client
            .sync(SyncSettings::default().token(initial_sync_token))
            .await
            .map_err(anyhow::Error::from)
    });

    info!("startup complete");
    tokio::try_join!(flatten(app_task), flatten(sync_task))?;

    Ok(())
}
