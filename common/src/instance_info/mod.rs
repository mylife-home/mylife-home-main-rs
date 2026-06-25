use kameo::{Actor, message, prelude::*};
use kameo_actors::scheduler::{Scheduler, SetInterval};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, SystemTime},
};

use crate::{
    bus::metadata::MetadataHandle,
    utils::actors::{ActorHandle, SpawnedActor, SpawnedActors},
};

pub mod types;

const INSTANCE_INFO_PUBLISHER_NAME: &str = "instance-info.publisher";

/// Client access to the instance-info publisher actor
#[derive(Debug, Clone)]
pub struct InstanceInfoPublisherHandle(ActorHandle<InstanceInfoPublisher>);

impl InstanceInfoPublisherHandle {
    /// Create a new access
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self(ActorHandle::from_name(INSTANCE_INFO_PUBLISHER_NAME)?))
    }

    /// Set type (ui, studio, ...)
    pub fn set_type(&self, name: &str) {
        self.0.send(SetType {
            name: name.to_owned(),
        });
    }

    /// Add component
    pub fn add_component(&self, name: &str, version: &str) {
        self.0.send(AddComponent {
            name: name.to_owned(),
            version: version.to_owned(),
        });
    }

    /// Add capability
    pub fn add_capability(&self, name: &str) {
        self.0.send(AddCapability {
            name: name.to_owned(),
        });
    }
}

pub async fn init_actors(actors: &mut SpawnedActors) {
    let (publisher, _) = SpawnedActor::start::<InstanceInfoPublisher>(()).await;

    publisher.register(INSTANCE_INFO_PUBLISHER_NAME);

    actors.add(publisher);
}

#[derive(Debug)]
struct InstanceInfoPublisher {
    scheduler: ActorRef<Scheduler>,
    metadata: MetadataHandle,

    r#type: Option<String>,
    components: HashMap<String, String>,
    capabilities: HashSet<String>,
    instance_uptime: SystemTime,
}

impl Actor for InstanceInfoPublisher {
    type Args = ();
    type Error = anyhow::Error;

    async fn on_start(_config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let metadata = MetadataHandle::new()?;

        let scheduler = Scheduler::spawn(Scheduler::new());

        let interval = SetInterval::new(actor_ref.downgrade(), Duration::from_secs(60), Refresh);
        scheduler.tell(interval).await?;

        Ok(Self {
            scheduler,
            metadata,
            r#type: None,
            components: HashMap::new(),
            capabilities: HashSet::new(),
            // Let's take actor startup time as instance uptime
            instance_uptime: SystemTime::now(),
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        self.scheduler.stop_gracefully().await?;
        self.scheduler.wait_for_shutdown().await;

        Ok(())
    }
}

impl InstanceInfoPublisher {
    fn refresh(&mut self) {
        let Some(r#type) = &self.r#type else {
            log::warn!("type not set, will not emit instance-info");
            return;
        };

        let info = types::InstanceInfo {
            r#type: r#type.clone(),
            hardware: HashMap::new(), // TODO
            versions: self.components.clone(),
            system_uptime: SystemTime::now(),
            instance_uptime: self.instance_uptime,
            hostname: String::from("TODO"), // TODO
            capabilities: self.capabilities.iter().cloned().collect(),

            wifi: None,
        };

        if let Err(e) = self.metadata.set("instance-info", &info) {
            log::error!("cannot set instance info: {}", e);
        }
    }
}

impl message::Message<SetType> for InstanceInfoPublisher {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetType,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.r#type = Some(msg.name);
        self.refresh();
    }
}

impl message::Message<AddComponent> for InstanceInfoPublisher {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: AddComponent,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.components.insert(msg.name, msg.version);
        self.refresh();
    }
}

impl message::Message<AddCapability> for InstanceInfoPublisher {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: AddCapability,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.capabilities.insert(msg.name);
        self.refresh();
    }
}

impl message::Message<Refresh> for InstanceInfoPublisher {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: Refresh,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.refresh();
    }
}

#[derive(Debug, Clone)]
struct Refresh;

#[derive(Debug, Clone)]
struct SetType {
    name: String,
}

#[derive(Debug, Clone)]
struct AddComponent {
    name: String,
    version: String,
}

#[derive(Debug, Clone)]
struct AddCapability {
    name: String,
}
