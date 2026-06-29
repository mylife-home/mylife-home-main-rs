use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::utils::{actors::SpawnedActors, config};

pub mod client;
pub mod encoding;
// pub mod logger;
pub mod metadata;
pub mod mqtt;

pub async fn init(actors: &mut SpawnedActors, instance_name: Arc<String>, listen_remote: bool) {
    let config = config::section::<BusConfig>("bus");

    client::init_pubsubs(actors).await;
    metadata::init_pubsubs(actors).await;

    client::init_actor(
        actors,
        client::ClientConfig {
            instance_name: instance_name.clone(),
            server_address: config.server_address,
        },
    )
    .await;

    metadata::init_actor(
        actors,
        metadata::MetadataConfig {
            instance_name: instance_name.clone(),
            listen_remote,
        },
    )
    .await;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BusConfig {
    server_address: String,
}
