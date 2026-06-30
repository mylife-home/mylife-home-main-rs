use std::sync::Arc;

use crate::utils::actors::SpawnedActors;

pub mod bus;
pub mod components;
pub mod instance_info;
pub mod utils;

#[derive(Debug)]
pub struct ActorsConfig {
    pub listen_remote_metadata: bool,
    pub listen_remote_logs: bool,
}

pub async fn init(actors: &mut SpawnedActors, r#type: &str, config: &ActorsConfig) {
    let hostname = utils::hostname().expect("could not read hostname");
    let instance_name = Arc::new(format!("{}-{}", hostname, r#type));

    bus::init(actors, instance_name.clone(), config).await;
    components::init(actors, instance_name.clone(), r#type).await;
}
