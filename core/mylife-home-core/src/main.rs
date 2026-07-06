use clap::Parser;
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

#[derive(Parser, Debug)]
#[command(name = "mylife-home-core")]
#[command(about = "Mylife Home Core")]
struct Cli {
    /// config file
    #[arg(long, default_value = "config.toml")]
    config: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    config::init(&cli.config);
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
