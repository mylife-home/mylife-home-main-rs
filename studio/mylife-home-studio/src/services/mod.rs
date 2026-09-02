use common::{InitData, utils::actors::SpawnedActors};

use crate::web::DispatcherBuilder;

mod git;
mod online;
mod project_manager;

pub async fn init(
    actors: &mut SpawnedActors,
    dispatcher: &mut DispatcherBuilder,
    init_data: &InitData,
) {
    online::init(actors, dispatcher, init_data).await;
    git::init(actors, dispatcher).await;
    project_manager::init(actors, dispatcher).await;
}
