use std::{collections::HashMap, time::Duration};

use tokio::time::sleep;

use common::{
    instance_info,
    utils::actors::SpawnedActors,
};
use plugin_runtime::runtime::{Config, ConfigValue};

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

    let server_address = "rpi-dev-home-main:1883".to_owned();

    let mut actors = SpawnedActors::new();

    common::init(&mut actors, "core", server_address).await;

    components::init_actor(&mut actors).await;
    components::init_plugins().await;

    let instance_info_handle = instance_info::InstanceInfoPublisherHandle::new();
    instance_info_handle.add_component("core", env!("CARGO_PKG_VERSION"));

    // build module list
    let mut modules = HashMap::new();

    for plugin in modules::registry().plugins() {
        let meta = plugin.metadata();
        modules.insert(meta.module(), meta.version());
    }

    for (name, version) in modules {
        instance_info_handle.add_component(&format!("core-plugin.{}", name), version);
    }

    create_component().await;

    sleep(Duration::from_secs(5000)).await;

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
