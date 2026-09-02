use kameo::{Actor, message, prelude::*};
use thiserror::Error;

use crate::{
    bus::metadata::MetadataHandle,
    utils::actors::{CallError, HandleLookupError, SpawnedActor, SpawnedActors},
};

use super::{InstanceInfoProviderHandle, types};

const INSTANCE_INFO_PUBLISHER_NAME: &str = "instance-info.publisher";

pub async fn init_actors(actors: &mut SpawnedActors) {
    let (publisher, _) = SpawnedActor::start::<InstanceInfoPublisher>(()).await;

    publisher.register(INSTANCE_INFO_PUBLISHER_NAME);

    actors.add(publisher);
}

#[derive(Debug)]
struct InstanceInfoPublisher {
    metadata: MetadataHandle,
}

/// Error that occurs when the instance info publisher actor fails to start or operate correctly.
#[derive(Debug, Error)]
pub enum InstanceInfoPublisherActorError {
    #[error("Failed to lookup actor handle: {0}")]
    HandleLookupError(#[from] HandleLookupError),
    #[error("Failed to set interval: {0}")]
    SchedulerError(#[from] CallError),
}

impl Actor for InstanceInfoPublisher {
    type Args = ();
    type Error = InstanceInfoPublisherActorError;

    async fn on_start(_config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let metadata = MetadataHandle::new()?;
        let provider = InstanceInfoProviderHandle::new()?;

        provider.on_event().subscribe(actor_ref);

        Ok(Self { metadata })
    }
}

impl message::Message<types::InstanceInfo> for InstanceInfoPublisher {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: types::InstanceInfo,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.metadata.set("instance-info", &msg, 0).await;
    }
}
