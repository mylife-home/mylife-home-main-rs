use std::{any::type_name, borrow::Cow, fmt, time::Duration};

use anyhow::Context;
use async_trait::async_trait;
use kameo::{
    Actor, Reply,
    actor::{ActorRef, Spawn, WeakActorRef},
    error::SendError,
    message,
};
use kameo_actors::{
    pubsub::{self, PubSub},
    scheduler::{self, Scheduler},
};
use tokio::task::AbortHandle;

const SCHEDULER_NAME: &str = "scheduler";

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
    pub fn from_name(name: impl Into<Cow<'static, str>>) -> anyhow::Result<Self> {
        let name = name.into();
        let actor_ref =
            ActorRef::lookup(&name)?.with_context(|| format!("actor '{}' not found", name))?;

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
        SendError<Message, <<Actor as message::Message<Message>>::Reply as Reply>::Error>,
    >
    where
        Actor: message::Message<Message>,
        Message: Send + 'static,
    {
        self.actor_ref.ask(msg).try_send().await
    }
}

///  PubSub specific handle
#[derive(Debug, Clone)]
pub struct PublisherHandle<Message: Send + Clone + 'static>(ActorHandle<PubSub<Message>>);

impl<Message: Send + Clone + 'static> PublisherHandle<Message> {
    /// Create a handle to a PubSub actor given its registry name
    pub fn from_name(name: impl Into<Cow<'static, str>>) -> anyhow::Result<Self> {
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
    pub fn from_name(name: impl Into<Cow<'static, str>>) -> anyhow::Result<Self> {
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
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self(ActorHandle::from_name(SCHEDULER_NAME)?))
    }

    pub async fn set_timeout<A, M>(
        &self,
        actor_ref: WeakActorRef<A>,
        duration: Duration,
        message: M,
    ) -> anyhow::Result<AbortHandle>
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
    ) -> anyhow::Result<AbortHandle>
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
        let actor_ref = TActor::spawn(args);

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
    fn register(&self, name: Cow<'static, str>) -> anyhow::Result<()>;
    async fn terminate(&self);
}

struct TypedSpawnedActor<TActor: Actor>(ActorRef<TActor>);

#[async_trait]
impl<TActor> UntypedSpawnedActor for TypedSpawnedActor<TActor>
where
    TActor: Actor,
    <TActor as Actor>::Error: fmt::Display,
{
    fn register(&self, name: Cow<'static, str>) -> anyhow::Result<()> {
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
