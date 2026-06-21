use std::{borrow::Cow, fmt, marker::PhantomData};

use anyhow::Context;
use async_trait::async_trait;
use kameo::{
    Actor,
    actor::{ActorRef, Spawn},
    message,
};
use kameo_actors::pubsub::{self, PubSub};

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
    /// Create a handle to an actor given its registry name
    pub fn from_name(name: impl Into<Cow<'static, str>>) -> anyhow::Result<Self> {
        let name = name.into();
        let actor_ref =
            ActorRef::lookup(&name)?.with_context(|| format!("actor '{}' not found", name))?;

        Ok(Self { name, actor_ref })
    }

    /// Synchronously send a message to an actor, and log on error
    pub fn tell_sync<Message>(&self, msg: Message)
    where
        Actor: message::Message<Message>,
        Message: Send + 'static,
    {
        if let Err(e) = self.actor_ref.tell(msg).try_send() {
            log::error!("Could not send message to actor '{}': {}", self.name, e);
        }
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
        self.0.tell_sync(pubsub::Publish(msg));
    }
}

pub async fn spawn_pubsub<Message: 'static>(name: &'static str) -> SpawnedActor {
    let (actor, _) = SpawnedActor::start::<pubsub::PubSub<Message>>(
        pubsub::PubSub::<Message>::new(kameo_actors::DeliveryStrategy::Guaranteed),
    )
    .await;

    actor.register(name);

    actor
}

#[derive(Actor)]
struct TracingActor<T: fmt::Debug + Send + 'static> {
    name: String,
    _data: PhantomData<T>,
}

impl<T: fmt::Debug + Send + 'static> message::Message<T> for TracingActor<T> {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: T,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        log::trace!("PubSub {} -> {:?}", self.name, msg);
    }
}

pub async fn trace_pubsub<T: fmt::Debug + Send + 'static>(name: &str) -> SpawnedActor {
    let pubsub = ActorRef::<pubsub::PubSub<T>>::lookup(name)
        .expect("lookup error")
        .expect("pubsub not found");

    let (tracer, tracer_ref) = SpawnedActor::start::<TracingActor<T>>(TracingActor::<T> {
        name: name.to_owned(),
        _data: PhantomData,
    })
    .await;

    pubsub
        .tell(pubsub::Subscribe(tracer_ref))
        .try_send()
        .expect("could not subscribe");

    tracer
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
                    panic!("could not start actor: {}", e);
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
        self.0.stop_gracefully().await.unwrap_or_else(|e| {
            log::error!("could not stop actor '{}': {}", "bus.client", e);
        });

        self.0
            .wait_for_shutdown_with_result(|res| {
                if let Err(e) = res {
                    log::error!("could not stop actor '{}': {}", "bus.client", e);
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
