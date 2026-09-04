use axum::{
    Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
    routing::get,
};
use common::utils::actors::SpawnExt;
use futures::{
    SinkExt, StreamExt,
    future::join_all,
    stream::{SplitSink, SplitStream},
};
use kameo::{
    Actor,
    error::{HookError, Infallible},
    mailbox::Signal,
    message,
    prelude::*,
};
use std::{
    collections::HashMap,
    fmt,
    hash::Hasher,
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
};
use std::{sync::Arc, time::Duration};
use studio_web_api::protocol;
use tokio::time::Instant;

use super::{AppState, Dispatcher, SessionEvent, SessionEventType};

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
pub struct SessionId(usize);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("#{}", self.0))
    }
}

#[derive(Debug)]
pub struct SessionManager {
    sessions: Mutex<HashMap<SessionId, SessionController>>,
    id_gen: AtomicUsize,
    dispatcher: Arc<Dispatcher>,
}

impl SessionManager {
    pub fn new(dispatcher: Arc<Dispatcher>) -> Self {
        SessionManager {
            sessions: Mutex::new(HashMap::new()),
            id_gen: AtomicUsize::new(1),
            dispatcher,
        }
    }

    /// Run a session. The method returns when the session is terminated
    pub async fn run(&self, socket: WebSocket) {
        let id = SessionId(self.id_gen.fetch_add(1, Ordering::Relaxed));

        tracing::debug!(%id, "websocket session started");

        let handle = SessionController::start(id, socket, self.dispatcher.clone()).await;

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

    fn sessions(&self) -> MutexGuard<'_, HashMap<SessionId, SessionController>> {
        self.sessions.lock().expect("could not lock sessions")
    }
}

#[derive(Debug, Clone, Hash)]
struct SessionController {
    id: SessionId,
    actor: ActorRef<Session>,
}

impl SessionController {
    /// Start session
    pub async fn start(id: SessionId, socket: WebSocket, dispatcher: Arc<Dispatcher>) -> Self {
        let actor = Session::spawn_unbounded((id, socket, dispatcher));

        if let Err(e) = actor.wait_for_startup_result().await {
            match e {
                HookError::Panicked(p) => {
                    panic!("session {} actor panicked at startup: {}", id, p);
                }
            }
        }

        Self { id, actor }
    }

    /// Terminate session
    pub async fn terminate(&self) {
        if let Err(error) = self.actor.stop_gracefully().await {
            tracing::error!(%error, session = ?self.id, "cannot stop session actor");
            return;
        }

        if let Err(e) = self.actor.wait_for_shutdown_result().await {
            match e {
                HookError::Panicked(p) => {
                    panic!("session {} actor panicked at shutdown: {}", self.id, p);
                }
                HookError::Error(error) => {
                    tracing::error!(%error, session = ?self.id, "session failed to shutdown");
                }
            }
        }
    }

    /// Wait for the end of the session
    pub async fn wait(&self) {
        self.actor.wait_for_shutdown().await;
    }
}

/// A handle to a session actor
#[derive(Debug, Clone)]
pub struct SessionHandle {
    actor: ActorRef<Session>,
    id: SessionId,
}

impl PartialEq for SessionHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for SessionHandle {}

impl std::hash::Hash for SessionHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl SessionHandle {
    fn new(actor: ActorRef<Session>, id: SessionId) -> Self {
        Self { actor, id }
    }

    /// Get the session ID
    pub fn id(&self) -> SessionId {
        self.id
    }

    /// Notify the session with a notification message. The notification is sent to the session actor, which will handle it asynchronously.
    pub fn notify<Data: serde::Serialize>(
        &self,
        notifier_type: &str,
        notifier_id: &str,
        data: &Data,
    ) {
        let notification = protocol::Notification {
            notifier_type: notifier_type.to_string(),
            notifier_id: notifier_id.to_string(),
            data: match serde_json::to_value(&data) {
                Ok(value) => value,
                Err(error) => {
                    tracing::error!(%error, session = ?self.id(), notifier_type, notifier_id, "failed to serialize notification data");
                    return;
                }
            },
        };

        if let Err(error) = self.actor.tell(notification).try_send() {
            tracing::error!(%error, session = ?self.id(), notifier_type, notifier_id, "could not send notification to session actor");
        }
    }

    /// Respond to a service request with a successful result.
    pub fn respond_ok<Data: serde::Serialize>(&self, request_id: &str, result: Data) {
        self.respond(protocol::ServiceResponse {
            request_id: request_id.to_owned(),
            result: serde_json::to_value(result).ok(),
            error: None,
        });
    }

    /// Respond to a service request with an error.
    pub fn respond_error<E: std::error::Error>(&self, request_id: &str, error: &E) {
        self.respond(protocol::ServiceResponse {
            request_id: request_id.to_owned(),
            result: None,
            error: Some(Dispatcher::format_error(error)),
        });
    }

    fn respond(&self, response: protocol::ServiceResponse) {
        let request_id = response.request_id.clone();
        if let Err(error) = self.actor.tell(response).try_send() {
            tracing::error!(%error, session = ?self.id(), request_id, "could not send service response to session actor");
        }
    }
}

struct Session {
    id: SessionId,
    ws_stream: SplitStream<WebSocket>,
    ws_sink: SplitSink<WebSocket, Message>,
    heartbeat: Heartbeat,
    dispatcher: Arc<Dispatcher>,
}

impl Actor for Session {
    type Args = (SessionId, WebSocket, Arc<Dispatcher>);
    type Error = Infallible;

    async fn on_start(
        (id, socket, dispatcher): Self::Args,
        actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        let (ws_sink, ws_stream) = socket.split();

        let mut _self = Self {
            id,
            ws_stream,
            ws_sink,
            heartbeat: Heartbeat::new(),
            dispatcher,
        };

        _self
            .dispatcher
            .session_event(SessionHandle::new(actor_ref, id), SessionEventType::Started);

        Ok(_self)
    }

    async fn on_stop(
        &mut self,
        actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        let actor_ref = actor_ref.upgrade().expect("actor ref should be valid");

        self.dispatcher.session_event(
            SessionHandle::new(actor_ref, self.id),
            SessionEventType::Stopped,
        );

        Ok(())
    }

    async fn next(
        &mut self,
        actor_ref: WeakActorRef<Self>,
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
                        Some(Ok(msg)) => {
                            self.handle_ws(&actor_ref, msg).await;
                        }
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

impl message::Message<protocol::Notification> for Session {
    type Reply = ();

    async fn handle(
        &mut self,
        notification: protocol::Notification,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.send_message(&protocol::ServerMessage::Notification(notification))
            .await;
    }
}

impl message::Message<protocol::ServiceResponse> for Session {
    type Reply = ();

    async fn handle(
        &mut self,
        response: protocol::ServiceResponse,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.send_message(&protocol::ServerMessage::ServiceResponse(response))
            .await;
    }
}

impl Session {
    async fn handle_ws(&mut self, actor_ref: &WeakActorRef<Self>, msg: Message) {
        tracing::trace!(session = ?self.id, ?msg, "<<");

        self.heartbeat.mark_alive();

        if let Message::Text(text) = &msg {
            let request = match serde_json::from_str::<protocol::ServiceRequest>(text) {
                Ok(request) => request,
                Err(error) => {
                    tracing::error!(%error, session = ?self.id, ?text, "failed to parse request");
                    return;
                }
            };

            let actor = actor_ref.upgrade().expect("actor ref should be valid");
            self.dispatcher
                .service_call(SessionHandle::new(actor, self.id), request);
        }
    }

    async fn send_message(&mut self, message: &protocol::ServerMessage) {
        let msg = match serde_json::to_string(&message) {
            Ok(data) => data,
            Err(error) => {
                tracing::error!(%error, session = ?self.id, ?message, "failed to serialize server message wrapper");
                return;
            }
        };

        self.send_raw(Message::text(msg)).await;
    }

    // TODO: doc recommands to use feed + flush to batch messages

    async fn send_raw(&mut self, msg: Message) {
        tracing::trace!(session = ?self.id, ?msg, ">>");

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
