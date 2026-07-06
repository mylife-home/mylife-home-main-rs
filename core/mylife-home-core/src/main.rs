use std::collections::HashMap;

use common::{
    ActorsConfig,
    bus::rpc::RpcHandle,
    instance_info,
    utils::{actors::SpawnedActors, config, hostname, logger, wait_for_shutdown_signal},
};

use crate::{components::ComponentConfig, store::StoreConfig};

mod bindings;
mod components;
mod modules;
mod store;

mod modules_include {
    #![allow(unused_imports)]
    use plugin_logic_base::*;
    use plugin_ui_base::*;
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

    store::init_actor(&mut actors, StoreConfig {}).await;
    components::init_actor(&mut actors).await;
    components::init_plugins().await;
    bindings::init_actor(&mut actors).await;

    let instance_info_handle = instance_info::InstanceInfoPublisherHandle::new();
    instance_info_handle.add_component("core", env!("CARGO_PKG_VERSION"));

    // let it connect
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    create_component().await;

    wait_for_shutdown_signal().await;

    actors.terminate().await;
}

async fn create_component() {
    let mut config = HashMap::new();
    config.insert("config".to_string(), serde_json::Value::from(false));

    let handle = RpcHandle::new().expect("failed to create rpc handle");
    let instance = hostname().expect("could not get hostname") + "-core";
    handle
        .call::<ComponentConfig, ()>(
            instance,
            "components.add",
            &ComponentConfig {
                id: "comp-id".to_owned(),
                plugin: "logic-base.value-binary".to_owned(),
                config,
            },
            None,
        )
        .await
        .expect("could not create component");
}
