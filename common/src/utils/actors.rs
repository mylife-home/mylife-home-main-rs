use std::{borrow::Cow, fmt};

use kameo::{actor::ActorRef, message};
use kameo_actors::pubsub::{self, PubSub};


pub struct ActorHandle<Actor: kameo::Actor> {
    name: Cow<'static, str>,
    actor_ref: ActorRef<Actor>,
}

impl<Actor: kameo::Actor> fmt::Debug for ActorHandle<Actor> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActorHandle").field("name", &self.name).field("actor_ref", &self.actor_ref).finish()
    }
}

impl<Actor: kameo::Actor> Clone for ActorHandle<Actor> {
    fn clone(&self) -> Self {
        Self { 
            name: self.name.clone(),
            actor_ref: self.actor_ref.clone()
        }
    }
}

impl<Actor: kameo::Actor> ActorHandle<Actor> {

    pub fn from_name(name: impl Into<Cow<'static, str>>) -> Self {
        let name = name.into();
        let actor_ref = ActorRef::lookup(&name)
            .expect("error during registry looking")
            .unwrap_or_else(|| panic!("actor '{}' not found", name));

        Self { name, actor_ref }
    }

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

#[derive(Debug, Clone)]
pub struct PublisherHandle<Message: Send + Clone + 'static>(ActorHandle<PubSub<Message>>);

impl<Message: Send + Clone + 'static> PublisherHandle<Message> {
    pub fn new(name: &'static str) -> Self {
        Self(ActorHandle::from_name(name))
    }

    pub fn publish(&self, msg: Message) {
        self.0.tell_sync(pubsub::Publish(msg));
    }
}
