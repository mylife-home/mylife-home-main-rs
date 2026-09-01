// project-manager/start-notify-list
// online/start-notify-status
// git/start-notify

use common::utils::actors::SpawnedActors;

use crate::web::DispatcherBuilder;

mod online;

pub async fn init(actors: &mut SpawnedActors, dispatcher: &mut DispatcherBuilder) {
    online::init_actor(actors).await;

    online::init_dispatcher(dispatcher).await;
}
