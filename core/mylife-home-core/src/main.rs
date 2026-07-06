use common::{
    ActorsConfig, instance_info,
    utils::{actors::SpawnedActors, config, logger, wait_for_shutdown_signal},
};

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
    // FIXME
    std::env::set_current_dir("./core/mylife-home-core").unwrap();

    config::init("config.toml");
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

    store::init_actor(&mut actors).await;
    components::init_plugins().await;
    components::init_actor(&mut actors).await;
    bindings::init_actor(&mut actors).await;

    let instance_info_handle = instance_info::InstanceInfoPublisherHandle::new();
    instance_info_handle.add_component("core", env!("CARGO_PKG_VERSION"));

    wait_for_shutdown_signal().await;

    actors.terminate().await;
}
