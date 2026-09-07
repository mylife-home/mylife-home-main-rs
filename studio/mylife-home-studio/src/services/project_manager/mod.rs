use common::utils::actors::{ActorHandle, HandleLookupError, SpawnedActor, SpawnedActors};
use kameo::{message, prelude::*};
use studio_web_api::{project_manager::UpdateListNotification, protocol};

use crate::web::{DispatcherBuilder, NotifierManager, ServiceRequest, SessionEvent};

const PROJECT_MANAGER_NAME: &str = "project-manager";

pub async fn init(actors: &mut SpawnedActors, dispatcher: &mut DispatcherBuilder) {
    let (project_manager, _) = SpawnedActor::start::<ProjectManager>(()).await;

    project_manager.register(PROJECT_MANAGER_NAME);

    actors.add(project_manager);

    let actor: ActorRef<_> = ActorHandle::<ProjectManager>::from_name(PROJECT_MANAGER_NAME)
        .expect("cannot get project manager actor handle")
        .into();

    dispatcher.register_session_handler(actor.clone());
    dispatcher
        .register_call::<StartNotifyListReq, _>("project-manager/start-notify-list", actor.clone());
    dispatcher.register_call::<StopNotifyListReq, _>("project-manager/stop-notify-list", actor);
}

#[derive(Debug, serde::Deserialize)]
struct StartNotifyListReq;

#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
struct StartNotifyListRes(protocol::NotifierId);

#[derive(Debug, serde::Deserialize)]
#[serde(transparent)]
struct StopNotifyListReq(protocol::NotifierId);

#[derive(Debug, serde::Serialize)]
struct StopNotifyListRes;

#[derive(Debug)]
struct ProjectManager {
    list_notifiers: NotifierManager<UpdateListNotification>,
}

impl Actor for ProjectManager {
    type Args = ();
    type Error = HandleLookupError;

    async fn on_start(_args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(Self {
            list_notifiers: NotifierManager::new("project-manager/list"),
        })
    }
}

impl message::Message<SessionEvent> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.list_notifiers.session_event(&msg);
    }
}

impl message::Message<ServiceRequest<StartNotifyListReq>> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<StartNotifyListReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
        let notifier = self.list_notifiers.create_notifier(call.session().clone());

        call.reply_ok(StartNotifyListRes(protocol::NotifierId {
            notifier_id: notifier.notifier_id().into(),
        }));
    }
}

impl message::Message<ServiceRequest<StopNotifyListReq>> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<StopNotifyListReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
        let notifier_id = &call.request().0;
        self.list_notifiers
            .remove_notifier(notifier_id.notifier_id.as_str());

        call.reply_ok(StopNotifyListRes);
    }
}
