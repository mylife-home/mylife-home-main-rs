use std::sync::Arc;

use crate::utils::actors::{SpawnedActors, spawn_scheduler};

pub mod bus;
pub mod components;
pub mod instance_info;
pub mod utils;

#[derive(Debug)]
pub struct ActorsConfig {
    pub listen_remote_metadata: bool,
    pub listen_remote_logs: bool,
}

#[derive(Debug)]
pub struct InitData {
    pub instance_name: Arc<String>,
}

pub async fn init(actors: &mut SpawnedActors, r#type: &str, config: &ActorsConfig) -> InitData {
    let hostname = utils::hostname().expect("could not read hostname");
    let instance_name = Arc::new(format!("{}-{}", hostname, r#type));

    actors.add(spawn_scheduler().await);

    instance_info::init_provider(actors).await;
    bus::init(actors, instance_name.clone(), config).await;
    components::init(actors, instance_name.clone(), r#type).await;
    instance_info::init_publisher(actors).await;

    InitData { instance_name }
}

pub async fn start() {
    instance_info::start().await;
    bus::start().await;
}
