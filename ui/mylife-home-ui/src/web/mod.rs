use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use common::{components::registry::RegistryHandle, utils::{actors::HandleLookupError, config}};
use serde::Deserialize;
use thiserror::Error;
use tokio::{io, net::TcpListener, task};
use tokio::sync::oneshot;

use crate::model::ModelHandle;

#[derive(Debug, Deserialize)]
struct WebConfig {
    listen_address: String,
}

#[derive(Debug, Clone)]
struct AppState {
    registry: RegistryHandle,
    model: ModelHandle,
}

#[derive(Debug)]
pub struct WebServer {
    shutdown: Option<oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Error)]
pub enum WebServerError {
    #[error("failed to lookup actor handle: {0}")]
    HandleLookupError(#[from] HandleLookupError),
    #[error("bind error: {0}")]
    BindError(#[source] io::Error),
}

impl WebServer {
    pub async fn new() -> Result<Self, WebServerError> {
        let config: WebConfig = config::section("web");
        let state = AppState { 
            registry: RegistryHandle::new()?,
            model: ModelHandle::new()?,
        };

        let app = Router::new()
            //.nest("/repository", repository_router())
            .nest("/resources", resources_router())
            //.route_service("/images/favicon.ico", ServeFile::new(favicon))
            //.fallback_service(static_service)
            .with_state(state);

        let listener = TcpListener::bind((config.listen_address)).await.map_err(WebServerError::BindError)?;

        let (tx, rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            let server = axum::serve(listener, app).with_graceful_shutdown(async {
                let _ = rx.await;
            });

            if let Err(error) = server.await {
                tracing::error!(?error, "web server error");
            }
        });

        Ok(Self {
            shutdown: Some(tx),
            task,
        })
    }

    pub async fn terminate(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }


        if let Err(error) = self.task.await {
            tracing::error!(?error, "could not join web server task");
        }
    }
}

// ----- repository routes -----------------------------------------------------
/*
fn repository_router() -> Router<AppState> {
    Router::new()
        .route("/action/{component_id}/{action_name}", get(action))
        .route("/components", get(components))
        .route("/state/{component_id}", get(state))
}

async fn action(
    State(state): State<AppState>,
    Path((component_id, action_name)): Path<(String, String)>,
) -> StatusCode {
    // execute true then false, as in the Node version
    state.registry.execute_action(&component_id, &action_name, true);
    state.registry.execute_action(&component_id, &action_name, false);
    StatusCode::OK
}

async fn components(State(state): State<AppState>) -> Json<Vec<String>> {
    let ids = state.registry.component_ids();
    Json(ids)
}

async fn state(
    State(state): State<AppState>,
    Path(component_id): Path<String>,
) -> Json<serde_json::Value> {
    let states = state.registry.component_states(&component_id);
    Json(states)
}
*/
// ----- resource routes -------------------------------------------------------

fn resources_router() -> Router<AppState> {
    // wildcard capture: /resources/<hash...>
    Router::new().route("/{*hash}", get(resource))
}

async fn resource(State(state): State<AppState>, Path(hash): Path<String>) -> Response {
    match state.model.get_resource(&hash).await {
        Ok(res) => (
            [
                (header::CONTENT_TYPE, res.mime()),
                (
                    header::CACHE_CONTROL,
                    "public, max-age=31557600, s-maxage=31557600", // 1 year
                ),
            ],
            res.data().clone(),
        )
            .into_response(),
        Err(err) => StatusCode::NOT_FOUND.into_response(),
    }
}
