use common::utils::actors::SpawnedActors;

use crate::web::DispatcherBuilder;

mod instances;
mod status;

pub async fn init(actors: &mut SpawnedActors, dispatcher: &mut DispatcherBuilder) {
    status::init(actors, dispatcher).await;
    instances::init(actors, dispatcher).await;
}
