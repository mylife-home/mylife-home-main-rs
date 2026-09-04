use kameo::{message, prelude::*};
use std::{any::type_name, collections::HashMap, fmt, sync::Arc};
use studio_web_api::protocol;
use thiserror::Error;

use super::SessionHandle;

#[derive(Debug, Error)]
pub enum DispatcherError {
    #[error("service not found: {0}")]
    ServiceNotFound(String),
    #[error("service actor unavailable: {0}")]
    ServiceUnavailable(String),
}

#[derive(Debug, Error)]
#[error("service call was dropped without a reply")]
pub struct UnansweredServiceCallError;

/// A session event, which can be sent to other actors to notify them of session lifecycle events.
#[derive(Debug, Clone)]
pub struct SessionEvent {
    session: SessionHandle,
    event_type: SessionEventType,
}

impl SessionEvent {
    /// Create a new session event.
    pub fn new(session: SessionHandle, event_type: SessionEventType) -> Self {
        Self {
            session,
            event_type,
        }
    }

    /// Get the session handle associated with this event.
    pub fn session(&self) -> &SessionHandle {
        &self.session
    }

    /// Get the type of event (started or stopped).
    pub fn event_type(&self) -> SessionEventType {
        self.event_type
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEventType {
    Started,
    Stopped,
}

#[derive(Debug)]
pub struct ServiceCall<Req: Send + 'static> {
    session: SessionHandle,
    request_id: String,
    request: Req,
    replied: bool,
}

impl<Req: Send + 'static> ServiceCall<Req> {
    fn new(session: SessionHandle, request_id: String, request: Req) -> Self {
        Self {
            session,
            request_id,
            request,
            replied: false,
        }
    }

    pub fn session(&self) -> &SessionHandle {
        &self.session
    }

    pub fn request(&self) -> &Req {
        &self.request
    }

    pub fn reply_ok<Res: serde::Serialize>(mut self, result: Res) {
        self.replied = true;
        self.session.respond_ok(&self.request_id, result);
    }

    pub fn reply_error<E: std::error::Error>(mut self, error: E) {
        self.replied = true;
        self.session.respond_error(&self.request_id, &error);
    }

    pub fn reply_result<Res: serde::Serialize, E: std::error::Error>(self, result: Result<Res, E>) {
        match result {
            Ok(result) => self.reply_ok(result),
            Err(error) => self.reply_error(error),
        }
    }
}

impl<Req: Send + 'static> Drop for ServiceCall<Req> {
    fn drop(&mut self) {
        if !self.replied {
            self.session
                .respond_error(&self.request_id, &UnansweredServiceCallError);
        }
    }
}

/// A dispatcher that routes service requests to the appropriate service handler and notifies session event recipients of session lifecycle events.
#[derive(Debug)]
pub struct Dispatcher {
    service_handlers: HashMap<String, Box<dyn ServiceHandler>>,
    session_handlers: Vec<Recipient<SessionEvent>>,
}

impl Dispatcher {
    /// Handle a service request by routing it to the appropriate service handler.
    pub fn service_call(&self, session: SessionHandle, request: protocol::ServiceRequest) {
        if let Some(handler) = self.service_handlers.get(&request.service) {
            handler.handle(session, request.request_id, request.payload);
        } else {
            tracing::error!(service = %request.service, "no service handler found");
            let err = DispatcherError::ServiceNotFound(request.service);
            session.respond_error(&request.request_id, &err);
        }
    }

    pub fn format_error<E: std::error::Error>(error: &E) -> protocol::ServiceResponseError {
        // capture the error chain
        let mut stacktrace = format!("{}", error);
        let mut source = error.source();
        while let Some(err) = source {
            stacktrace.push_str(&format!("\ncaused by: {}", err));
            source = err.source();
        }

        protocol::ServiceResponseError {
            r#type: type_name::<E>().to_string(),
            message: format!("{}", error),
            stack: stacktrace,
        }
    }

    /// Notify all registered session event recipients of a session lifecycle event.
    pub fn session_event(&self, session: SessionHandle, event_type: SessionEventType) {
        let event = SessionEvent::new(session, event_type);

        for recipient in &self.session_handlers {
            if let Err(error) = recipient.tell(event.clone()).try_send() {
                tracing::error!(%error, session_id = %event.session.id(), event_type = ?event.event_type, "could not send session event to actor");
            }
        }
    }
}

/// DispatcherBuilder is used to build a Dispatcher with service handlers and session event recipients.
#[derive(Debug)]
pub struct DispatcherBuilder {
    service_handlers: HashMap<String, Box<dyn ServiceHandler>>,
    session_handlers: Vec<Recipient<SessionEvent>>,
}

impl DispatcherBuilder {
    /// Create a new DispatcherBuilder.
    pub fn new() -> Self {
        Self {
            service_handlers: HashMap::new(),
            session_handlers: Vec::new(),
        }
    }

    /// Build a Dispatcher with the provided service handlers and session event recipients.
    pub fn build(self) -> Arc<Dispatcher> {
        Arc::new(Dispatcher {
            service_handlers: self.service_handlers,
            session_handlers: self.session_handlers,
        })
    }

    /// Register an actor to receive session lifecycle events (started/stopped).
    pub fn register_session_handler<A: Actor + Message<SessionEvent> + 'static>(
        &mut self,
        actor: ActorRef<A>,
    ) {
        let recipient = actor.recipient();
        self.session_handlers.push(recipient);
    }

    /// Register a service call handler for a specific service name.
    pub fn register_call<
        Req: serde::de::DeserializeOwned + Send + 'static,
        A: Actor + message::Message<ServiceCall<Req>, Reply = ()> + Send + 'static,
    >(
        &mut self,
        service_name: impl Into<String>,
        actor: ActorRef<A>,
    ) {
        let recipient = actor.recipient::<ServiceCall<Req>>();
        let handler = ServiceHandlerImpl(recipient);
        self.service_handlers
            .insert(service_name.into(), Box::new(handler));
    }
}

trait ServiceHandler: Send + Sync + fmt::Debug + 'static {
    fn handle(&self, session: SessionHandle, request_id: String, request: serde_json::Value);
}

struct ServiceHandlerImpl<Req: serde::de::DeserializeOwned + Send + 'static>(
    Recipient<ServiceCall<Req>>,
);

impl<Req: serde::de::DeserializeOwned + Send + 'static> fmt::Debug for ServiceHandlerImpl<Req> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceHandlerImpl").finish()
    }
}

impl<Req: serde::de::DeserializeOwned + Send + 'static> ServiceHandler for ServiceHandlerImpl<Req> {
    fn handle(&self, session: SessionHandle, request_id: String, request: serde_json::Value) {
        let req = match serde_json::from_value::<Req>(request) {
            Ok(req) => req,
            Err(error) => {
                session.respond_error(&request_id, &error);
                return;
            }
        };

        let call = ServiceCall::new(session.clone(), request_id.clone(), req);
        if let Err(error) = self.0.tell(call).try_send() {
            tracing::error!(%error, "could not dispatch service call");
            session.respond_error(
                &request_id,
                &DispatcherError::ServiceUnavailable(error.to_string()),
            );
        }
    }
}
