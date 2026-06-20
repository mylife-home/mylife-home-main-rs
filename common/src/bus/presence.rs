use std::{collections::HashSet, sync::Arc};

use kameo::{error::Infallible, message, prelude::*};

use crate::{
    bus::{client, encoding},
    utils::actors::PublisherHandle,
};

/// Name of the PubSub actor that delivers online changes
pub const ONLINE_PUBSUB_NAME: &str = "bus.presence.online";

const DOMAIN: &str = "online";

#[derive(Debug)]
pub struct Presence {
    instance_name: Arc<String>,
    online_instances: HashSet<String>,

    on_online: PublisherHandle<Online>,
}

pub struct PresenceConfig {
    pub instance_name: Arc<String>,
}

impl Actor for Presence {
    type Args = PresenceConfig;
    type Error = Infallible;

    async fn on_start(config: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(Self {
            instance_name: config.instance_name,
            online_instances: HashSet::new(),
            on_online: PublisherHandle::from_name(ONLINE_PUBSUB_NAME),
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        self.online_instances.clear();

        Ok(())
    }
}

impl message::Message<client::Online> for Presence {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: client::Online,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if msg.is_online() {
            return;
        }

        for instance in self.online_instances.drain() {
            self.on_online.publish(Online::new(Arc::new(instance), false));
        }
    }
}

impl message::Message<client::Message> for Presence {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: client::Message,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let (Some(domain), Some(instance)) = (msg.domain(), msg.instance()) else {
            return;
        };

        if domain != DOMAIN || instance == self.instance_name.as_str() {
            return;
        }

        let online = match encoding::read_bool(msg.payload()) {
            Ok(value) => value,
            Err(e) => {
                log::error!("Error reading online value ({:?}): {}", msg.payload(), e);
                return;
            }
        };

        self.set_online(String::from(instance), online);
    }
}

impl Presence {
    fn set_online(&mut self, instance: String, value: bool) {
        let do_publish = if value {
            self.online_instances.insert(instance.clone())
        } else {
            self.online_instances.remove(&instance)
        };

        if do_publish {
            self.on_online
                .publish(Online::new(Arc::new(instance), value));
        }
    }
}

#[derive(Debug, Clone)]
pub struct Online {
    instance: Arc<String>,
    value: bool,
}

impl Online {
    fn new(instance: Arc<String>, value: bool) -> Self {
        Self { instance, value }
    }

    pub fn instance(&self) -> &str {
        &self.instance
    }

    pub fn value(&self) -> bool {
        self.value
    }
}
