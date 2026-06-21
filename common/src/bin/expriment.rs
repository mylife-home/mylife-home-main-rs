use std::{sync::Arc, time::Duration};

use common::{
    bus::client,
    utils::actors::{SpawnedActor, SpawnedActors, spawn_pubsub, trace_pubsub},
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

    actors.add(trace_pubsub::<client::InstanceOnline>(client::INSTANCE_ONLINE_PUBSUB_NAME).await);
    actors.add(trace_pubsub::<client::Online>(client::ONLINE_PUBSUB_NAME).await);
    actors.add(trace_pubsub::<client::Message>(client::MESSAGE_PUBSUB_NAME).await);

    let (client, _) = SpawnedActor::start::<client::Client>(client::ClientConfig {
        instance_name,
        server_address,
    })
    .await;

    client.register("bus.client");

    actors.add(client);

    sleep(Duration::from_secs(10)).await;
    // shutdown

    actors.terminate().await;
}
