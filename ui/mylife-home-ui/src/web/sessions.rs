use axum::{
    Router,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::IntoResponse,
    routing::get,
};
use futures::{StreamExt, future::join_all};
use kameo::{
    Actor,
    error::{HookError, Infallible},
    prelude::*,
};
use std::{
    collections::HashMap,
    fmt,
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
};

use super::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(ws_handler))
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    state.sessions.run(socket).await;
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct SessionId(usize);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("#{}", self.0))
    }
}

#[derive(Debug)]
pub struct SessionManager {
    sessions: Mutex<HashMap<SessionId, SessionHandle>>,
    id_gen: AtomicUsize,
}

impl SessionManager {
    pub fn new() -> Self {
        SessionManager {
            sessions: Mutex::new(HashMap::new()),
            id_gen: AtomicUsize::new(1),
        }
    }

    /// Run a session. The method returns when the session is terminated
    pub async fn run(&self, socket: WebSocket) {
        let id = SessionId(self.id_gen.fetch_add(1, Ordering::Relaxed));

        tracing::debug!(%id, "websocket session started");

        let handle = match SessionHandle::start(id, socket).await {
            Ok(handle) => handle,
            Err(error) => {
                tracing::error!(?error, %id, "error starting websocket session");
                return;
            }
        };

        self.sessions().insert(id, handle.clone());

        // keep this Axum-side task alive until the session stops
        handle.wait().await;

        self.sessions().remove(&id);

        tracing::debug!(%id, "websocket session terminated");
    }

    /// Shutdown all sessions
    pub async fn shutdown(&self) {
        let handles: Vec<_> = self.sessions().values().cloned().collect();
        join_all(handles.iter().map(|handle| handle.terminate())).await;
    }

    fn sessions(&self) -> MutexGuard<'_, HashMap<SessionId, SessionHandle>> {
        self.sessions.lock().expect("could not lock sessions")
    }
}

#[derive(Debug, Clone, Hash)]
struct SessionHandle {
    id: SessionId,
    actor: ActorRef<Session>,
}

impl SessionHandle {
    /// Start session
    pub async fn start(id: SessionId, socket: WebSocket) -> Result<Self, Infallible> {
        let actor = Session::spawn((id, socket));

        if let Err(e) = actor.wait_for_startup_result().await {
            match e {
                HookError::Panicked(p) => {
                    panic!("session {} actor panicked at startup: {}", id, p);
                }
                HookError::Error(e) => {
                    // TODO
                    return Err(e);
                }
            }
        }

        Ok(Self { id, actor })
    }

    /// Terminate session
    pub async fn terminate(&self) {
        if let Err(error) = self.actor.stop_gracefully().await {
            tracing::error!(?error, id = %self.id, "cannot stop session actor");
            return;
        }

        if let Err(e) = self.actor.wait_for_shutdown_result().await {
            match e {
                HookError::Panicked(p) => {
                    panic!("session {} actor panicked at shutdown: {}", self.id, p);
                }
                HookError::Error(error) => {
                    tracing::error!(?error, id = %self.id, "session failed to shutdown");
                }
            }
        }
    }

    /// Wait for the end of the session
    pub async fn wait(&self) {
        self.actor.wait_for_shutdown().await;
    }
}

struct Session {
    id: SessionId,
}

impl Actor for Session {
    type Args = (SessionId, WebSocket);
    type Error = Infallible;

    async fn on_start(
        (id, socket): Self::Args,
        actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        let (sink, stream) = socket.split();

        todo!()
    }
}
