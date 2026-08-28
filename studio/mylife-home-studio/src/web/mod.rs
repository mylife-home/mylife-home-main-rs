use std::sync::Arc;

use axum::Router;
use common::utils::{actors::HandleLookupError, config};
use serde::Deserialize;
use thiserror::Error;
use tokio::{io, net::TcpListener, sync::oneshot};

use crate::web::sessions::SessionManager;

mod dispatcher;
mod sessions;
mod webapp;

pub use dispatcher::{Dispatcher, DispatcherBuilder, SessionEvent, SessionEventType};
pub use sessions::{SessionHandle, SessionId};

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
    pub async fn new(dispatcher: Arc<Dispatcher>) -> Result<Self, WebServerError> {
        let config: WebConfig = config::section("web");
        let state = AppState {
            sessions: Arc::new(SessionManager::new(dispatcher)),
        };

        let app = Router::new()
            .nest("/websocket", sessions::router())
            .merge(webapp::router())
            .with_state(state.clone());

        let listener = TcpListener::bind(config.listen_address)
            .await
            .map_err(WebServerError::BindError)?;

        let (tx, rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            let server = axum::serve(listener, app).with_graceful_shutdown(async {
                let _ = rx.await;
            });

            if let Err(error) = server.await {
                tracing::error!(%error, "web server error");
            }

            state.sessions.shutdown().await;
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
            tracing::error!(%error, "could not join web server task");
        }
    }
}

#[derive(Debug, Clone)]
struct AppState {
    sessions: Arc<SessionManager>,
}
