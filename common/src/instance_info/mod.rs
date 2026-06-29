use kameo::{Actor, message, prelude::*};
use kameo_actors::scheduler::{Scheduler, SetInterval};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    time::{Duration, SystemTime},
};

use crate::{
    bus::metadata::MetadataHandle,
    utils::{
        self,
        actors::{ActorHandle, SpawnedActor, SpawnedActors},
    },
};

pub mod types;

const INSTANCE_INFO_PUBLISHER_NAME: &str = "instance-info.publisher";

/// Client access to the instance-info publisher actor
#[derive(Debug, Clone)]
pub struct InstanceInfoPublisherHandle(ActorHandle<InstanceInfoPublisher>);

impl InstanceInfoPublisherHandle {
    /// Create a new access
    pub fn new() -> Self {
        // Needed at init, so we can fail on error
        Self(
            ActorHandle::from_name(INSTANCE_INFO_PUBLISHER_NAME)
                .expect("could not get instance info publisher handler"),
        )
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
    versions: HashMap<String, String>,
    capabilities: HashSet<String>,
    instance_uptime: SystemTime,
    hardware_info: HashMap<String, String>,
}

impl Actor for InstanceInfoPublisher {
    type Args = ();
    type Error = anyhow::Error;

    async fn on_start(_config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let metadata = MetadataHandle::new()?;

        let scheduler = Scheduler::spawn(Scheduler::new());

        let interval = SetInterval::new(actor_ref.downgrade(), Duration::from_secs(60), Refresh);
        scheduler.tell(interval).await?;

        let mut versions = HashMap::new();

        if let Some(version) = Self::os_version() {
            versions.insert("os".to_owned(), version);
        }

        if let Some(version) = Self::kernel_version() {
            versions.insert("kernel".to_owned(), version);
        }

        Ok(Self {
            scheduler,
            metadata,
            r#type: None,
            versions,
            capabilities: HashSet::new(),
            // Let's take actor startup time as instance uptime
            instance_uptime: SystemTime::now(),
            hardware_info: Self::get_hardware_info(),
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
            tracing::warn!("type not set, will not emit instance-info");
            return;
        };

        let hostname = match utils::hostname() {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, "could not read hostname");
                "<unknown>".to_owned()
            }
        };

        let info = types::InstanceInfo {
            r#type: r#type.clone(),
            hardware: self.hardware_info.clone(),
            versions: self.versions.clone(),
            system_uptime: SystemTime::now(),
            instance_uptime: self.instance_uptime,
            hostname,
            capabilities: self.capabilities.iter().cloned().collect(),

            wifi: None,
        };

        if let Err(error) = self.metadata.set("instance-info", &info) {
            tracing::error!(?error, "cannot set instance info");
        }
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
                tracing::debug!(?error, "could not read /proc/cpuinfo");
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
                tracing::error!(?error, "could not read /etc/os-release");
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
                tracing::error!(?error, "could not read /proc/sys/kernel/osrelease");
                None
            }
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
        self.versions.insert(msg.name, msg.version);
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
