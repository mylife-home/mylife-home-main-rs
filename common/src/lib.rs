use std::sync::Arc;

use crate::utils::actors::SpawnedActors;

pub mod bus;
pub mod components;
pub mod instance_info;
pub mod utils;

pub async fn init(actors: &mut SpawnedActors, r#type: &str, listen_remote: bool) {
    let hostname = utils::hostname().expect("could not read hostname");
    let instance_name = Arc::new(format!("{}-{}", hostname, r#type));

    bus::init(actors, instance_name.clone(), listen_remote).await;
    components::init(actors, instance_name.clone(), r#type).await;
}
