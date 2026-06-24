use std::{sync::Arc, time::Duration};

use tokio::time::sleep;

use common::{
    bus::{client, metadata},
    components::{registry, remote},
    utils::actors::SpawnedActors,
};
use plugin_runtime::runtime::{Config, ConfigValue, Value};

use crate::components::LocalComponentsHandle;

mod components;
mod modules;

mod modules_include {
    #![allow(unused_imports)]
    use plugin_logic_base::*;
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    modules::init();

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

    components::init_actor(&mut actors).await;
    components::init_plugins().await;

    create_component().await;

    sleep(Duration::from_secs(5)).await;

    delete_component().await;

    sleep(Duration::from_secs(5)).await;

    // shutdown
    actors.terminate().await;
}

async fn create_component() {
    let mut config = Config::new();
    config.insert("config".to_string(), ConfigValue::Bool(false));

    let handle = LocalComponentsHandle::new().expect("failed to create handle");

    handle
        .component_add(
            "comp-id".to_owned(),
            "logic-base.value-binary".to_owned(),
            config,
        )
        .await
        .expect("could not create component");
}

async fn delete_component() {
    let handle = LocalComponentsHandle::new().expect("failed to create handle");

    handle
        .component_remove("comp-id".to_owned())
        .await
        .expect("could not delete component");
}
