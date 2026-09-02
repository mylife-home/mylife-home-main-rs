use kameo::{Actor, message, prelude::*};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    time::{Duration, Instant},
};
use thiserror::Error;

use crate::utils::{
    self,
    actors::{
        ActorHandle, CallError, HandleLookupError, PublisherHandle, SchedulerHandle, SpawnedActor,
        SpawnedActors, SubscriberHandle, spawn_pubsub,
    },
    system_uptime,
};

use super::types;

/// Name of the instance-info provider actor
const INSTANCE_INFO_PROVIDER_NAME: &str = "instance-info.provider";

/// Name of the PubSub actor that delivers events
const EVENT_PUBSUB_NAME: &str = "instance-info.provider.event";

/// Client access to the instance-info provider actor
#[derive(Debug, Clone)]
pub struct InstanceInfoProviderHandle {
    actor: ActorHandle<InstanceInfoProvider>,
    on_event: SubscriberHandle<types::InstanceInfo>,
}

impl InstanceInfoProviderHandle {
    /// Create a new access
    pub fn new() -> Result<Self, HandleLookupError> {
        Ok(Self {
            actor: ActorHandle::from_name(INSTANCE_INFO_PROVIDER_NAME)?,
            on_event: SubscriberHandle::from_name(EVENT_PUBSUB_NAME)?,
        })
    }

    /// Create a new access without failure
    pub fn new_safe() -> Self {
        Self::new().expect("failed to access instance-info provider")
    }

    /// Set type (ui, studio, ...)
    pub fn set_type(&self, name: &str) {
        self.actor.send(SetType {
            name: name.to_owned(),
        });
    }

    /// Add component
    pub fn add_component(&self, name: &str, version: &str) {
        self.actor.send(AddComponent {
            name: name.to_owned(),
            version: version.to_owned(),
        });
    }

    /// Add capability
    pub fn add_capability(&self, name: &str) {
        self.actor.send(AddCapability {
            name: name.to_owned(),
        });
    }

    /// Get the PubSub for incoming MQTT messages
    pub fn on_event(&self) -> &SubscriberHandle<types::InstanceInfo> {
        &self.on_event
    }
}

pub async fn init_pubsubs(actors: &mut SpawnedActors) {
    actors.add(spawn_pubsub::<types::InstanceInfo>(EVENT_PUBSUB_NAME).await);
}

pub async fn init_actors(actors: &mut SpawnedActors) {
    let (provider, _) = SpawnedActor::start::<InstanceInfoProvider>(()).await;

    provider.register(INSTANCE_INFO_PROVIDER_NAME);

    actors.add(provider);
}

#[derive(Debug)]
struct InstanceInfoProvider {
    on_event: PublisherHandle<types::InstanceInfo>,

    r#type: Option<String>,
    versions: HashMap<String, String>,
    capabilities: HashSet<String>,
    instance_uptime: Instant,
    hardware_info: HashMap<String, String>,
}

/// Error that occurs when the instance info provider actor fails to start or operate correctly.
#[derive(Debug, Error)]
pub enum InstanceInfoProviderActorError {
    #[error("Failed to lookup actor handle: {0}")]
    HandleLookupError(#[from] HandleLookupError),
    #[error("Failed to set interval: {0}")]
    SchedulerError(#[from] CallError),
}

impl Actor for InstanceInfoProvider {
    type Args = ();
    type Error = InstanceInfoProviderActorError;

    async fn on_start(_config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let scheduler = SchedulerHandle::new()?;

        scheduler
            .set_interval(actor_ref.downgrade(), Duration::from_secs(60), Refresh)
            .await?;

        let mut versions = HashMap::new();

        if let Some(version) = Self::os_version() {
            versions.insert("os".to_owned(), version);
        }

        if let Some(version) = Self::kernel_version() {
            versions.insert("kernel".to_owned(), version);
        }

        Ok(Self {
            on_event: PublisherHandle::from_name(EVENT_PUBSUB_NAME)?,
            r#type: None,
            versions,
            capabilities: HashSet::new(),
            // Let's take actor startup time as instance uptime
            instance_uptime: Instant::now(),
            hardware_info: Self::get_hardware_info(),
        })
    }
}

impl InstanceInfoProvider {
    async fn refresh(&mut self) {
        let Some(r#type) = &self.r#type else {
            tracing::warn!("type not set, will not emit instance-info");
            return;
        };

        let hostname = match utils::hostname() {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(%error, "could not read hostname");
                "<unknown>".to_owned()
            }
        };

        let system_uptime = match system_uptime() {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(%error, "could not read uptime");
                Duration::ZERO
            }
        };

        let info = types::InstanceInfo {
            r#type: r#type.clone(),
            hardware: self.hardware_info.clone(),
            versions: self.versions.clone(),
            system_uptime,
            instance_uptime: self.instance_uptime.elapsed(),
            hostname,
            capabilities: self.capabilities.iter().cloned().collect(),

            wifi: None,
        };

        self.on_event.publish(info);
    }

    fn get_hardware_info() -> HashMap<String, String> {
        let mut hardware = HashMap::new();

        let info = match rpi_info::load_cpuinfo() {
            Ok(Some(info)) => info,
            Ok(None) => {
                // not a recognized Pi
                hardware.insert("main".to_owned(), env::consts::ARCH.to_owned());
                return hardware;
            }
            Err(error) => {
                tracing::debug!(%error, "could not read /proc/cpuinfo");
                hardware.insert("main".to_owned(), env::consts::ARCH.to_owned());
                return hardware;
            }
        };

        hardware.insert(
            "main".to_owned(),
            format!("Raspberry Pi {}", info.revision.model),
        );
        hardware.insert(
            "processor".to_owned(),
            format!("{:?}", info.revision.processor),
        );
        hardware.insert(
            "memory".to_owned(),
            format!("{} MB", info.revision.memory.mib()),
        );
        hardware.insert("manufacturer".to_owned(), format!("{}", info.revision.mfg));

        hardware
    }

    fn os_version() -> Option<String> {
        let content = match fs::read_to_string("/etc/os-release") {
            Ok(content) => content,
            Err(error) => {
                tracing::error!(%error, "could not read /etc/os-release");
                return None;
            }
        };

        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                if key == "PRETTY_NAME" {
                    return Some(value.trim().trim_matches('"').to_owned());
                }
            }
        }

        tracing::error!("no PRETTY_NAME field in /etc/os-release");
        None
    }

    /// Returns the running kernel version from /proc/sys/kernel/osrelease
    /// (same as `uname -r`), e.g. "6.6.31+rpt-rpi-v8". Logs and returns None on failure.
    fn kernel_version() -> Option<String> {
        match fs::read_to_string("/proc/sys/kernel/osrelease") {
            Ok(content) => Some(content.trim_end().to_owned()),
            Err(error) => {
                tracing::error!(%error, "could not read /proc/sys/kernel/osrelease");
                None
            }
        }
    }
}

impl message::Message<SetType> for InstanceInfoProvider {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetType,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.r#type = Some(msg.name);
        self.refresh().await;
    }
}

impl message::Message<AddComponent> for InstanceInfoProvider {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: AddComponent,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.versions.insert(msg.name, msg.version);
        self.refresh().await;
    }
}

impl message::Message<AddCapability> for InstanceInfoProvider {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: AddCapability,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.capabilities.insert(msg.name);
        self.refresh().await;
    }
}

impl message::Message<Refresh> for InstanceInfoProvider {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: Refresh,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.refresh().await;
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
