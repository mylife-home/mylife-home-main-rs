use async_trait::async_trait;
use kameo::{message, prelude::*};
use std::{any::type_name, collections::HashMap, fmt, sync::Arc};
use studio_web_api::protocol;
use thiserror::Error;

use super::SessionHandle;

#[derive(Debug, Error)]
pub enum DispatcherError {
    #[error("service not found: {0}")]
    ServiceNotFound(String),
}

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
pub struct ServiceRequest<Req: serde::de::DeserializeOwned + Send + 'static> {
    session: SessionHandle,
    request: Req,
}

impl<Req: serde::de::DeserializeOwned + Send + 'static> ServiceRequest<Req> {
    /// Create a new service request.
    pub fn new(session: SessionHandle, request: Req) -> Self {
        Self { session, request }
    }

    /// Get the session handle associated with this request.
    pub fn session(&self) -> &SessionHandle {
        &self.session
    }

    /// Get the request payload.
    pub fn request(&self) -> &Req {
        &self.request
    }
}

impl<Req: serde::de::DeserializeOwned + Send + 'static> From<ServiceRequest<Req>>
    for (SessionHandle, Req)
{
    fn from(req: ServiceRequest<Req>) -> Self {
        (req.session, req.request)
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
    pub async fn service_call(
        &self,
        session: SessionHandle,
        request: protocol::ServiceRequest,
    ) -> protocol::ServiceResponse {
        let res = if let Some(handler) = self.service_handlers.get(&request.service) {
            handler.handle(session, request.payload).await
        } else {
            tracing::error!(service = %request.service, "no service handler found");
            let err = DispatcherError::ServiceNotFound(request.service);
            Err(Self::format_error(&err))
        };

        let mut response = protocol::ServiceResponse {
            request_id: request.request_id,
            result: None,
            error: None,
        };

        match res {
            Ok(result) => {
                response.result = Some(result);
            }
            Err(error) => {
                response.error = Some(error);
            }
        }

        response
    }

    fn format_error<E: std::error::Error>(error: &E) -> protocol::ServiceResponseError {
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
        A: Actor + message::Message<ServiceRequest<Req>> + Send + 'static,
    >(
        &mut self,
        service_name: impl Into<String>,
        actor: ActorRef<A>,
    ) where
        <<A as message::Message<ServiceRequest<Req>>>::Reply as Reply>::Ok: serde::Serialize,
        <<A as message::Message<ServiceRequest<Req>>>::Reply as Reply>::Error:
            std::error::Error + Send + Sync,
    {
        let recipient = actor.reply_recipient::<ServiceRequest<Req>>();
        let handler = ServiceHandlerImpl(recipient);
        self.service_handlers
            .insert(service_name.into(), Box::new(handler));
    }
}

#[async_trait]
trait ServiceHandler: Send + Sync + fmt::Debug + 'static {
    async fn handle(
        &self,
        session: SessionHandle,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, protocol::ServiceResponseError>;
}

struct ServiceHandlerImpl<
    Req: serde::de::DeserializeOwned + Send + 'static,
    Res: serde::Serialize + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
>(ReplyRecipient<ServiceRequest<Req>, Res, E>);

impl<
    Req: serde::de::DeserializeOwned + Send + 'static,
    Res: serde::Serialize + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
> fmt::Debug for ServiceHandlerImpl<Req, Res, E>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceHandlerImpl").finish()
    }
}

#[async_trait]
impl<
    Req: serde::de::DeserializeOwned + Send + 'static,
    Res: serde::Serialize + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
> ServiceHandler for ServiceHandlerImpl<Req, Res, E>
{
    async fn handle(
        &self,
        session: SessionHandle,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, protocol::ServiceResponseError> {
        let req = match serde_json::from_value::<Req>(request) {
            Ok(req) => req,
            Err(error) => {
                return Err(Dispatcher::format_error(&error));
            }
        };

        let value = match self.0.ask(ServiceRequest::new(session, req)).await {
            Ok(result) => result,
            Err(error) => {
                return Err(Dispatcher::format_error(&error));
            }
        };

        let value = match serde_json::to_value(value) {
            Ok(value) => value,
            Err(error) => {
                return Err(Dispatcher::format_error(&error));
            }
        };

        Ok(value)
    }
}
