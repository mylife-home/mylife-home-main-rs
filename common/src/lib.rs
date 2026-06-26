use std::sync::Arc;

use crate::{
    bus::{client, metadata},
    components::{registry, remote},
    utils::actors::SpawnedActors,
};

pub mod bus;
pub mod components;
pub mod instance_info;
pub mod utils;

pub async fn init(actors: &mut SpawnedActors, r#type: &str, server_address: String) {
    let hostname = utils::hostname().expect("could not read hostname");

    let instance_name = Arc::new(format!("{}-{}", hostname, r#type));

    client::init_pubsubs(actors).await;
    metadata::init_pubsubs(actors).await;
    registry::init_pubsubs(actors).await;

    client::init_actor(
        actors,
        client::ClientConfig {
            instance_name: instance_name.clone(),
            server_address,
        },
    )
    .await;

    metadata::init_actor(
        actors,
        metadata::MetadataConfig {
            instance_name: instance_name.clone(),
            listen_remote: true,
        },
    )
    .await;

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
