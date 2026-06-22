use std::{sync::Arc, time::Duration};

use common::{
    bus::{client, metadata},
    utils::actors::{SpawnedActor, SpawnedActors, spawn_pubsub},
};
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let instance_name = Arc::new("common-demo-client".to_owned());
    let server_address = "rpi-dev-home-main:1883".to_owned();

    let mut actors = SpawnedActors::new();

    actors.add(spawn_pubsub::<client::InstanceOnline>(client::INSTANCE_ONLINE_PUBSUB_NAME).await);
    actors.add(spawn_pubsub::<client::Online>(client::ONLINE_PUBSUB_NAME).await);
    actors.add(spawn_pubsub::<client::Message>(client::MESSAGE_PUBSUB_NAME).await);

    let (client, _) = SpawnedActor::start::<client::Client>(client::ClientConfig {
        instance_name: instance_name.clone(),
        server_address,
    })
    .await;

    client.register("bus.client");

    actors.add(client);

    actors.add(spawn_pubsub::<metadata::RemoteMetadataUpdate>(metadata::REMOTE_METADATA_SET_PUBSUB_NAME).await);

    let (metadata, _) = SpawnedActor::start::<metadata::Metadata>(metadata::MetadataConfig {
        instance_name: instance_name.clone(),
        listen_remote: true,
    })
    .await;

    metadata.register("bus.metadata");

    actors.add(metadata);

    sleep(Duration::from_secs(10)).await;
    // shutdown

    actors.terminate().await;
}
