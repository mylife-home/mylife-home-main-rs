use axum::{
    Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
    routing::get,
};
use common::utils::actors::{CallError, HandleLookupError};
use futures::{
    SinkExt, StreamExt,
    future::join_all,
    stream::{SplitSink, SplitStream},
};
use kameo::{Actor, error::HookError, mailbox::Signal, message, prelude::*};
use std::{
    collections::HashMap,
    fmt,
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
};
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tokio::time::Instant;

use super::AppState;

const IDLE_BEFORE_PING: Duration = Duration::from_secs(30);
const PONG_TIMEOUT: Duration = Duration::from_secs(5);

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
                tracing::error!(%error, %id, "error starting websocket session");
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

#[derive(Debug, Error)]
#[error("failed to start session '{id}': {error}")]
struct SessionStartError {
    id: SessionId,
    error: Arc<SessionActorError>,
}

impl SessionStartError {
    pub fn new(id: SessionId, error: Arc<SessionActorError>) -> Self {
        Self { id, error }
    }
}

#[derive(Debug, Clone, Hash)]
struct SessionHandle {
    id: SessionId,
    actor: ActorRef<Session>,
}

impl SessionHandle {
    /// Start session
    pub async fn start(id: SessionId, socket: WebSocket) -> Result<Self, SessionStartError> {
        let actor = Session::spawn((id, socket));

        if let Err(e) = actor.wait_for_startup_result().await {
            match e {
                HookError::Panicked(p) => {
                    panic!("session {} actor panicked at startup: {}", id, p);
                }
                HookError::Error(e) => {
                    return Err(SessionStartError::new(id, e));
                }
            }
        }

        Ok(Self { id, actor })
    }

    /// Terminate session
    pub async fn terminate(&self) {
        if let Err(error) = self.actor.stop_gracefully().await {
            tracing::error!(%error, session = %self.id, "cannot stop session actor");
            return;
        }

        if let Err(e) = self.actor.wait_for_shutdown_result().await {
            match e {
                HookError::Panicked(p) => {
                    panic!("session {} actor panicked at shutdown: {}", self.id, p);
                }
                HookError::Error(error) => {
                    tracing::error!(%error, session = %self.id, "session failed to shutdown");
                }
            }
        }
    }

    /// Wait for the end of the session
    pub async fn wait(&self) {
        self.actor.wait_for_shutdown().await;
    }
}

#[derive(Debug, Error)]
pub enum SessionActorError {
    #[error("failed to lookup actor handle: {0}")]
    HandleLookupError(#[source] HandleLookupError),
    #[error("failed to call model: {0}")]
    ModelCallError(#[source] CallError),
}

struct Session {
    id: SessionId,
    ws_stream: SplitStream<WebSocket>,
    ws_sink: SplitSink<WebSocket, Message>,
    heartbeat: Heartbeat,
}

impl Actor for Session {
    type Args = (SessionId, WebSocket);
    type Error = Arc<SessionActorError>;

    async fn on_start(
        (id, socket): Self::Args,
        actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        let (ws_sink, ws_stream) = socket.split();

        let mut _self = Self {
            id,
            ws_stream,
            ws_sink,
            heartbeat: Heartbeat::new(),
        };

        // _self.registry.on_update().subscribe(actor_ref.clone());

        _self.init().await?;

        Ok(_self)
    }

    async fn next(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        mailbox_rx: &mut MailboxReceiver<Self>,
    ) -> Result<Option<Signal<Self>>, Self::Error> {
        loop {
            let heartbeat = tokio::time::sleep_until(self.heartbeat.next_deadline());
            tokio::pin!(heartbeat);

            tokio::select! {
                signal = mailbox_rx.recv() => {
                    return Ok(signal);
                }

                ws_msg = self.ws_stream.next() => {
                    match ws_msg {
                        Some(Ok(msg)) => { self.handle_ws(msg).await; }
                        Some(Err(e)) => {
                            tracing::error!(error = %e, session = ?self.id, "ws stream error, stopping session");
                            return Ok(Some(Signal::Stop));
                        }
                        None => {
                            tracing::debug!(session = ?self.id, "ws stream ended, stopping session");
                            return Ok(Some(Signal::Stop));
                        }
                    }
                }

                _ = &mut heartbeat => {
                    match self.heartbeat.on_elapsed() {
                        HeartbeatAction::Ping => {
                            self.send_raw(Message::Ping(Vec::new().into())).await;
                        }
                        HeartbeatAction::Stop => {
                            tracing::debug!(session = ?self.id, "pong timeout, stopping session");
                            return Ok(Some(Signal::Stop));
                        }
                        HeartbeatAction::None => {},
                    }
                }
            }
        }
    }
}

impl Session {
    async fn init(&mut self) -> Result<(), SessionActorError> {
        Ok(())
    }

    async fn handle_ws(&mut self, msg: Message) {
        tracing::trace!(session = %self.id, ?msg, "<<");

        self.heartbeat.mark_alive();

        if let Message::Text(text) = &msg {
            // TODO
        }
    }

    // TODO: doc recommands to use feed + flush to batch messages

    async fn send_raw(&mut self, msg: Message) {
        tracing::trace!(session = %self.id, ?msg, ">>");

        if let Err(error) = self.ws_sink.send(msg).await {
            tracing::error!(%error, "ws send error");
        }
    }
}

/// What the session should do when the heartbeat deadline elapses.
enum HeartbeatAction {
    /// Send a ping; peer has been idle.
    Ping,
    /// Pong overdue; peer is dead, stop the session.
    Stop,
    /// Nothing due yet (spurious wake); carry on.
    None,
}

/// Tracks connection liveness via two deadlines: when to ping after idle,
/// and when an awaited pong must arrive. Owns no I/O.
#[derive(Debug)]
struct Heartbeat {
    /// None = not awaiting a pong; Some = deadline by which one must arrive.
    pong_deadline: Option<Instant>,
    /// When to ping next if no traffic arrives before then.
    idle_deadline: Instant,
}

impl Heartbeat {
    fn new() -> Self {
        Self {
            pong_deadline: None,
            idle_deadline: Instant::now() + IDLE_BEFORE_PING,
        }
    }

    /// The next instant the heartbeat should wake: the sooner of the pong
    /// timeout (if awaiting one) and the idle-before-ping deadline.
    fn next_deadline(&self) -> Instant {
        match self.pong_deadline {
            Some(pong) => pong.min(self.idle_deadline),
            None => self.idle_deadline,
        }
    }

    /// Called when the deadline elapses; decides the action and advances state.
    fn on_elapsed(&mut self) -> HeartbeatAction {
        let now = Instant::now();

        if let Some(deadline) = self.pong_deadline {
            if now >= deadline {
                return HeartbeatAction::Stop;
            }
        }

        if now >= self.idle_deadline {
            self.pong_deadline = Some(now + PONG_TIMEOUT);
            self.idle_deadline = now + IDLE_BEFORE_PING;
            return HeartbeatAction::Ping;
        }

        HeartbeatAction::None
    }

    /// Any inbound traffic (including the pong) proves the peer is alive.
    fn mark_alive(&mut self) {
        let now = Instant::now();
        self.idle_deadline = now + IDLE_BEFORE_PING;
        self.pong_deadline = None;
    }
}
