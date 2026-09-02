use crate::utils::actors::SpawnedActors;

mod provider;
mod publisher;
pub mod types;

pub use provider::InstanceInfoProviderHandle;

pub async fn init(actors: &mut SpawnedActors) {
    provider::init_pubsubs(actors).await;

    provider::init_actors(actors).await;
    publisher::init_actors(actors).await;
}
