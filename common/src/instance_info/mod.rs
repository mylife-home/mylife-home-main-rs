use crate::utils::actors::SpawnedActors;

mod provider;
mod publisher;
pub mod types;

pub use provider::InstanceInfoProviderHandle;

pub async fn init_provider(actors: &mut SpawnedActors) {
    provider::init_pubsubs(actors).await;
    provider::init_actors(actors).await;
}

pub async fn init_publisher(actors: &mut SpawnedActors) {
    publisher::init_actors(actors).await;
}

pub async fn start() {
    provider::start().await;
}
