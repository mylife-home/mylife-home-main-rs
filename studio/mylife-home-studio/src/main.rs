use clap::Parser;
use common::{
    ActorsConfig, instance_info,
    utils::{actors::SpawnedActors, config, logger, wait_for_shutdown_signal},
};

use crate::web::WebServer;

mod services;
mod web;

#[derive(Parser, Debug)]
#[command(name = "mylife-home-studio")]
#[command(about = "Mylife Home Studio")]
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
        "studio",
        &ActorsConfig {
            listen_remote_metadata: true,
            listen_remote_logs: true,
        },
    )
    .await;

    let instance_info_handle = instance_info::InstanceInfoPublisherHandle::new();
    instance_info_handle.add_component("studio", env!("CARGO_PKG_VERSION"));

    let dispatcher = web::DispatcherBuilder::new();
    // TODO: Register service handlers with the dispatcher here.
    let web = WebServer::new(dispatcher.build())
        .await
        .expect("could not start web server");

    wait_for_shutdown_signal().await;

    web.terminate().await;

    actors.terminate().await;
}
