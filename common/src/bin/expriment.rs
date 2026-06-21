use core::panic;
use std::{sync::Arc, time::Duration};

use common::bus::client;
use kameo::actor::Spawn;
use kameo_actors::pubsub;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let instance_name = Arc::new("common-demo-client".to_owned());
    let server_address = "rpi-dev-home-main:1883".to_owned();

    spawn_pubsub::<client::InstanceOnline>(client::INSTANCE_ONLINE_PUBSUB_NAME).await;
    spawn_pubsub::<client::Online>(client::ONLINE_PUBSUB_NAME).await;
    spawn_pubsub::<client::Message>(client::MESSAGE_PUBSUB_NAME).await;

    let client_ref = client::Client::spawn(client::ClientConfig {
        instance_name,
        server_address,
    });

    client_ref
        .wait_for_startup_with_result(|res| {
            if let Err(e) = res {
                panic!("could not start actor '{}': {}", "bus.client", e);
            }
        })
        .await;

    client_ref.register("bus.client").unwrap_or_else(|e| {
        panic!("could not register actor '{}': {}", "bus.client", e);
    });

    sleep(Duration::from_secs(10)).await;
    // shutdown

    client_ref.stop_gracefully().await.unwrap_or_else(|e| {
        panic!("could not stop actor '{}': {}", "bus.client", e);
    });

    client_ref
        .wait_for_shutdown_with_result(|res| {
            if let Err(e) = res {
                panic!("could not stop actor '{}': {}", "bus.client", e);
            }
        })
        .await;
}

async fn spawn_pubsub<Message: 'static>(name: &'static str) {
    let actor_ref = pubsub::PubSub::spawn(pubsub::PubSub::<Message>::new(
        kameo_actors::DeliveryStrategy::Guaranteed,
    ));

    actor_ref.register(name).unwrap_or_else(|e| {
        panic!("could not register actor '{}': {}", name, e);
    });

    actor_ref
        .wait_for_startup_with_result(|res| {
            if let Err(e) = res {
                panic!("could not start actor '{}': {}", name, e);
            }
        })
        .await;
}
