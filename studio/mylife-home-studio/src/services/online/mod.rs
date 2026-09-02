use common::{InitData, utils::actors::SpawnedActors};

use crate::web::DispatcherBuilder;

mod components;
mod instances;
mod status;

pub async fn init(
    actors: &mut SpawnedActors,
    dispatcher: &mut DispatcherBuilder,
    init_data: &InitData,
) {
    status::init(actors, dispatcher).await;
    instances::init(actors, dispatcher, init_data).await;
    components::init(actors, dispatcher).await;
}
