use std::sync::Arc;

use crate::{instance_info, utils::actors::SpawnedActors};

pub mod metadata;
pub mod registry;
pub mod remote;
pub mod types;

pub async fn init(actors: &mut SpawnedActors, instance_name: Arc<String>, r#type: &str) {
    registry::init_pubsubs(actors).await;

    registry::init_actor(actors).await;
    remote::init_actor(
        actors,
        remote::RemoteConfig {
            instance_name: instance_name.clone(),
        },
    )
    .await;

    instance_info::init_actors(actors).await;

    let instance_info_handle = instance_info::InstanceInfoPublisherHandle::new();
    instance_info_handle.set_type(r#type);
    instance_info_handle.add_component("common", env!("CARGO_PKG_VERSION"));
}
