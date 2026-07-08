use clap::Parser;
use common::{
    ActorsConfig, instance_info,
    utils::{actors::SpawnedActors, config, logger, wait_for_shutdown_signal},
};

mod model;
mod web;

#[derive(Parser, Debug)]
#[command(name = "mylife-home-ui")]
#[command(about = "Mylife Home UI")]
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

    let mut actors = SpawnedActors::new().await;

    common::init(
        &mut actors,
        "ui",
        &ActorsConfig {
            listen_remote_metadata: true,
            listen_remote_logs: false,
        },
    )
    .await;

    model::init_pubsubs(&mut actors).await;
    model::init_actor(&mut actors).await;

    let instance_info_handle = instance_info::InstanceInfoPublisherHandle::new();
    instance_info_handle.add_component("ui", env!("CARGO_PKG_VERSION"));

    wait_for_shutdown_signal().await;

    actors.terminate().await;
}
