// project-manager/start-notify-list
// online/start-notify-status
// git/start-notify

use common::utils::actors::SpawnedActors;

use crate::web::DispatcherBuilder;

mod git;
mod online;
mod project_manager;

pub async fn init(actors: &mut SpawnedActors, dispatcher: &mut DispatcherBuilder) {
    online::init_actor(actors).await;
    git::init_actor(actors).await;
    project_manager::init_actor(actors).await;

    online::init_dispatcher(dispatcher);
    git::init_dispatcher(dispatcher);
    project_manager::init_dispatcher(dispatcher);
}
