use std::sync::Arc;

use serde::Deserialize;

use crate::{
    ActorsConfig,
    utils::{actors::SpawnedActors, config},
};

pub mod client;
pub mod encoding;
pub mod logger;
pub mod metadata;
pub mod mqtt;
pub mod rpc;

pub async fn init(actors: &mut SpawnedActors, instance_name: Arc<String>, config: &ActorsConfig) {
    let file_config = config::section::<BusConfig>("bus");

    client::init_pubsubs(actors).await;
    metadata::init_pubsubs(actors).await;
    logger::init_pubsubs(actors).await;

    client::init_actor(
        actors,
        client::ClientConfig {
            instance_name: instance_name.clone(),
            server_address: file_config.server_address,
        },
    )
    .await;

    metadata::init_actor(
        actors,
        metadata::MetadataConfig {
            instance_name: instance_name.clone(),
            listen_remote: config.listen_remote_metadata,
        },
    )
    .await;

    logger::init_actor(
        actors,
        logger::LoggerConfig {
            instance_name: instance_name.clone(),
            listen_remote: config.listen_remote_logs,
        },
    )
    .await;

    rpc::init_actor(
        actors,
        rpc::RpcConfig {
            instance_name: instance_name.clone(),
        },
    )
    .await;
}

#[derive(Debug, Clone, Deserialize)]
struct BusConfig {
    server_address: String,
}
