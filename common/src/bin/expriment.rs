use std::{sync::Arc, time::Duration};

use common::{
    bus::{client, metadata},
    components::{registry, remote},
    utils::actors::SpawnedActors,
};
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let instance_name = Arc::new("common-demo-client".to_owned());
    let server_address = "rpi-dev-home-main:1883".to_owned();

    let mut actors = SpawnedActors::new();

    client::init_pubsubs(&mut actors).await;
    metadata::init_pubsubs(&mut actors).await;
    registry::init_pubsubs(&mut actors).await;

    client::init_actor(
        &mut actors,
        client::ClientConfig {
            instance_name: instance_name.clone(),
            server_address,
        },
    )
    .await;

    metadata::init_actor(
        &mut actors,
        metadata::MetadataConfig {
            instance_name: instance_name.clone(),
            listen_remote: true,
        },
    )
    .await;

    registry::init_actor(&mut actors).await;
    remote::init_actor(&mut actors).await;

    sleep(Duration::from_secs(10)).await;
    // shutdown

    actors.terminate().await;
}
