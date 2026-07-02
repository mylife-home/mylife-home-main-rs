use std::collections::HashMap;

use common::{
    ActorsConfig, instance_info,
    utils::{actors::SpawnedActors, config, logger, wait_for_shutdown_signal},
};

use crate::{components::{ComponentConfig, LocalComponentsHandle}, store::StoreConfig};

mod components;
mod modules;
mod store;

mod modules_include {
    #![allow(unused_imports)]
    use plugin_logic_base::*;
}

#[tokio::main]
async fn main() {
    config::init("core.toml");
    logger::init();
    modules::init();

    let mut actors = SpawnedActors::new().await;

    common::init(
        &mut actors,
        "core",
        &ActorsConfig {
            listen_remote_metadata: true,
            listen_remote_logs: false,
        },
    )
    .await;

    store::init_actor(&mut actors, StoreConfig{}).await;
    components::init_actor(&mut actors).await;
    components::init_plugins().await;

    let instance_info_handle = instance_info::InstanceInfoPublisherHandle::new();
    instance_info_handle.add_component("core", env!("CARGO_PKG_VERSION"));

    create_component().await;

    wait_for_shutdown_signal().await;

    delete_component().await;

    actors.terminate().await;
}

async fn create_component() {
    let mut config = HashMap::new();
    config.insert("config".to_string(), serde_json::Value::from(false));

    let handle = LocalComponentsHandle::new().expect("failed to create handle");

    handle
        .component_add(ComponentConfig {
            id: "comp-id".to_owned(),
            plugin: "logic-base.value-binary".to_owned(),
            config,
        })
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
