use axum::{
    Json, Router, http::StatusCode, response::{IntoResponse, Response},
};
use common::{
    components::registry::RegistryHandle,
    utils::{actors::HandleLookupError, config},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{io, net::TcpListener, sync::oneshot};

use crate::model::ModelHandle;

mod repository;
mod resources;
mod webapp;

#[derive(Debug, Deserialize)]
struct WebConfig {
    listen_address: String,
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
            .nest("/repository", repository::router())
            .nest("/resources", resources::router())
            .merge(webapp::setup())
            .with_state(state);

        let listener = TcpListener::bind(config.listen_address)
            .await
            .map_err(WebServerError::BindError)?;

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

#[derive(Debug, Clone)]
struct AppState {
    registry: RegistryHandle,
    model: ModelHandle,
}

#[derive(Debug, Serialize)]
struct WebError {
    error: String,
}

impl<E: std::error::Error> From<E> for WebError {
    fn from(value: E) -> Self {
        WebError {
            error: format!("{}", value),
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response()
    }
}
