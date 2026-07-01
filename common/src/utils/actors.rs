use std::{any::type_name, borrow::Cow, fmt, time::Duration};

use async_trait::async_trait;
use kameo::{
    Actor, Reply,
    actor::{ActorRef, Spawn, WeakActorRef},
    error::{Infallible, RegistryError},
    mailbox, message,
};
use kameo_actors::{
    pubsub::{self, PubSub},
    scheduler::{self, Scheduler},
};
use thiserror::Error;
use tokio::task::AbortHandle;

const SCHEDULER_NAME: &str = "scheduler";

/// Error that occurs when looking up an actor handle by name
#[derive(Debug, Error)]
pub enum HandleLookupError {
    #[error("registry error: {0}")]
    RegistryError(#[from] RegistryError),
    #[error("actor '{0}' not found")]
    ActorNotFound(String),
}

#[derive(Debug, Error)]
pub enum CallError<E = Infallible> {
    /// The actor isn't running.
    #[error("actor is not running")]
    ActorNotRunning,
    /// The actor panicked or was stopped before a reply could be received.
    #[error("actor stopped before reply could be received")]
    ActorStopped,
    /// The actors mailbox is full.
    #[error("actor's mailbox is full")]
    MailboxFull,
    /// An error returned by the actor's message handler.
    #[error("error in actor's message handler: {0}")]
    HandlerError(#[from] E),
    /// Timed out waiting for a reply.
    #[error("timeout waiting for reply")]
    Timeout,
}

impl<A, E> From<kameo::error::SendError<A, E>> for CallError<E> {
    fn from(value: kameo::error::SendError<A, E>) -> Self {
        match value {
            kameo::error::SendError::ActorNotRunning(_) => CallError::ActorNotRunning,
            kameo::error::SendError::ActorStopped => CallError::ActorStopped,
            kameo::error::SendError::MailboxFull(_) => CallError::MailboxFull,
            kameo::error::SendError::HandlerError(e) => CallError::HandlerError(e),
            kameo::error::SendError::Timeout(_) => CallError::Timeout,
        }
    }
}

/// Handle to an actor
pub struct ActorHandle<Actor: kameo::Actor> {
    name: Cow<'static, str>,
    actor_ref: ActorRef<Actor>,
}

impl<Actor: kameo::Actor> fmt::Debug for ActorHandle<Actor> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActorHandle")
            .field("name", &self.name)
            .field("actor_ref", &self.actor_ref)
            .finish()
    }
}

impl<Actor: kameo::Actor> Clone for ActorHandle<Actor> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            actor_ref: self.actor_ref.clone(),
        }
    }
}

impl<Actor: kameo::Actor> ActorHandle<Actor> {
    /// Create a handle to an actor given its ref
    pub fn from_ref(actor_ref: ActorRef<Actor>, name: impl Into<Cow<'static, str>>) -> Self {
        let name = name.into();
        Self { name, actor_ref }
    }

    /// Create a handle to an actor given its registry name
    pub fn from_name(name: impl Into<Cow<'static, str>>) -> Result<Self, HandleLookupError> {
        let name = name.into();
        let actor_ref = ActorRef::lookup(&name)?
            .ok_or_else(|| HandleLookupError::ActorNotFound(name.to_string()))?;

        Ok(Self { name, actor_ref })
    }

    /// Synchronously send a message to an actor, and log on error
    pub fn send<Message>(&self, msg: Message)
    where
        Actor: message::Message<Message>,
        Message: Send + 'static,
    {
        if let Err(error) = self.actor_ref.tell(msg).try_send() {
            tracing::error!(?error, name = %self.name, "could not send message to actor");
        }
    }

    /// Call the actor, waiting for its reply
    pub async fn call<Message>(
        &self,
        msg: Message,
    ) -> Result<
        <<Actor as message::Message<Message>>::Reply as Reply>::Ok,
        CallError<<<Actor as message::Message<Message>>::Reply as Reply>::Error>,
    >
    where
        Actor: message::Message<Message>,
        Message: Send + 'static,
    {
        let res = self.actor_ref.ask(msg).try_send().await?;
        Ok(res)
    }
}

///  PubSub specific handle
#[derive(Debug, Clone)]
pub struct PublisherHandle<Message: Send + Clone + 'static>(ActorHandle<PubSub<Message>>);

impl<Message: Send + Clone + 'static> PublisherHandle<Message> {
    /// Create a handle to a PubSub actor given its registry name
    pub fn from_name(name: impl Into<Cow<'static, str>>) -> Result<Self, HandleLookupError> {
        Ok(Self(ActorHandle::from_name(name)?))
    }

    /// Publish a message to the PubSub
    pub fn publish(&self, msg: Message) {
        self.0.send(pubsub::Publish(msg));
    }
}

#[derive(Debug, Clone)]
pub struct SubscriberHandle<M: Send + Clone + 'static>(ActorHandle<PubSub<M>>);

impl<M: Send + Clone + 'static> SubscriberHandle<M> {
    /// Create a handle to a PubSub actor given its registry name
    pub fn from_name(name: impl Into<Cow<'static, str>>) -> Result<Self, HandleLookupError> {
        Ok(Self(ActorHandle::from_name(name)?))
    }

    /// Subscribe to the PubSub
    pub fn subscribe<A: Actor + message::Message<M>>(&self, actor_ref: ActorRef<A>) {
        self.0.send(pubsub::Subscribe(actor_ref));
    }
}

pub async fn spawn_pubsub<Message: 'static>(name: &'static str) -> SpawnedActor {
    let (actor, _) = SpawnedActor::start::<PubSub<Message>>(PubSub::<Message>::new(
        kameo_actors::DeliveryStrategy::Guaranteed,
    ))
    .await;

    actor.register(name);

    actor
}

#[derive(Debug, Clone)]
pub struct SchedulerHandle(ActorHandle<Scheduler>);

impl SchedulerHandle {
    /// Create a handle to the scheduler
    pub fn new() -> Result<Self, HandleLookupError> {
        Ok(Self(ActorHandle::from_name(SCHEDULER_NAME)?))
    }

    pub async fn set_timeout<A, M>(
        &self,
        actor_ref: WeakActorRef<A>,
        duration: Duration,
        message: M,
    ) -> Result<AbortHandle, CallError>
    where
        A: Actor + message::Message<M>,
        M: Send + Sync + 'static,
    {
        let handle = self
            .0
            .call(scheduler::SetTimeout::new(actor_ref, duration, message))
            .await?;
        Ok(handle)
    }

    pub async fn set_interval<A, M>(
        &self,
        actor_ref: WeakActorRef<A>,
        duration: Duration,
        message: M,
    ) -> Result<AbortHandle, CallError>
    where
        A: Actor + message::Message<M>,
        M: Send + Sync + Clone + 'static,
    {
        let handle = self
            .0
            .call(scheduler::SetInterval::new(actor_ref, duration, message))
            .await?;
        Ok(handle)
    }
}

pub async fn spawn_scheduler() -> SpawnedActor {
    let (actor, _) = SpawnedActor::start::<Scheduler>(Scheduler::new()).await;

    actor.register(SCHEDULER_NAME);

    actor
}

pub struct SpawnedActor(Box<dyn UntypedSpawnedActor>);

impl SpawnedActor {
    pub async fn start<TActor>(args: TActor::Args) -> (Self, ActorRef<TActor>)
    where
        TActor: Actor,
        <TActor as Actor>::Error: fmt::Display,
    {
        let actor_ref = TActor::spawn_with_mailbox(args, mailbox::unbounded());

        actor_ref
            .wait_for_startup_with_result(|res| {
                if let Err(e) = res {
                    panic!("could not start actor '{}': {}", type_name::<TActor>(), e);
                }
            })
            .await;

        (
            Self(Box::new(TypedSpawnedActor(actor_ref.clone()))),
            actor_ref,
        )
    }

    pub fn register(&self, name: impl Into<Cow<'static, str>>) {
        self.0
            .register(name.into())
            .unwrap_or_else(|e| panic!("could not register actor: {}", e));
    }

    pub async fn terminate(&self) {
        self.0.terminate().await;
    }
}

#[async_trait]
trait UntypedSpawnedActor {
    fn register(&self, name: Cow<'static, str>) -> Result<(), RegistryError>;
    async fn terminate(&self);
}

struct TypedSpawnedActor<TActor: Actor>(ActorRef<TActor>);

#[async_trait]
impl<TActor> UntypedSpawnedActor for TypedSpawnedActor<TActor>
where
    TActor: Actor,
    <TActor as Actor>::Error: fmt::Display,
{
    fn register(&self, name: Cow<'static, str>) -> Result<(), RegistryError> {
        self.0.register(name)?;

        Ok(())
    }

    async fn terminate(&self) {
        self.0.stop_gracefully().await.unwrap_or_else(|error| {
            tracing::error!(?error, name = type_name::<TActor>(), "could not stop actor");
        });

        self.0
            .wait_for_shutdown_with_result(|res| {
                if let Err(error) = res {
                    tracing::error!(?error, name = type_name::<TActor>(), "could not stop actor");
                }
            })
            .await;
    }
}

pub struct SpawnedActors(Vec<SpawnedActor>);

impl SpawnedActors {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add(&mut self, actor: SpawnedActor) {
        self.0.push(actor);
    }

    pub async fn terminate(&mut self) {
        for actor in self.0.iter().rev() {
            actor.terminate().await;
        }

        self.0.clear();
    }
}
