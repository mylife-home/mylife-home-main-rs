use std::path::PathBuf;

use common::utils::actors::{ActorHandle, HandleLookupError, SpawnedActor, SpawnedActors};
use kameo::{message, prelude::*};
use serde::{Serialize, de::DeserializeOwned};
use studio_web_api::{
    project_manager::{self, UpdateListNotification},
    protocol,
};

use crate::{
    services::project_manager::fs_collection::FsCollection,
    web::{DispatcherBuilder, NotifierManager, ServiceRequest, SessionEvent},
};

mod fs_collection;

const PROJECT_MANAGER_NAME: &str = "project-manager";

pub async fn init(
    actors: &mut SpawnedActors,
    dispatcher: &mut DispatcherBuilder,
    config: ProjectManagerConfig,
) {
    let (project_manager, _) = SpawnedActor::start::<ProjectManager>(config).await;

    project_manager.register(PROJECT_MANAGER_NAME);

    actors.add(project_manager);

    let actor: ActorRef<_> = ActorHandle::<ProjectManager>::from_name(PROJECT_MANAGER_NAME)
        .expect("cannot get project manager actor handle")
        .into();

    dispatcher.register_session_handler(actor.clone());

    dispatcher
        .register_call::<StartNotifyListReq, _>("project-manager/start-notify-list", actor.clone());
    dispatcher
        .register_call::<StopNotifyListReq, _>("project-manager/stop-notify-list", actor.clone());
    dispatcher.register_call::<CreateNewReq, _>("project-manager/create-new", actor.clone());
    dispatcher.register_call::<DuplicateReq, _>("project-manager/duplicate", actor.clone());
    dispatcher.register_call::<RenameReq, _>("project-manager/rename", actor.clone());
    dispatcher.register_call::<DeleteReq, _>("project-manager/delete", actor.clone());
    dispatcher.register_call::<OpenReq, _>("project-manager/open", actor.clone());
    dispatcher.register_call::<CloseReq, _>("project-manager/close", actor.clone());
    dispatcher.register_call::<CallOpenedReq, _>("project-manager/call-opened", actor.clone());

    // TODO
    // Services.instance.git.registerPathFeature('project/core', paths.core);
    // Services.instance.git.registerPathFeature('project/ui', paths.ui);
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

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateNewReq {
    r#type: project_manager::ProjectType,
    id: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateNewRes {
    r#type: project_manager::ProjectType,
    created_id: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DuplicateReq {
    r#type: project_manager::ProjectType,
    id: String,
    new_id: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DuplicateRes {
    r#type: project_manager::ProjectType,
    created_id: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameReq {
    r#type: project_manager::ProjectType,
    id: String,
    new_id: String,
}

#[derive(Debug, serde::Serialize)]
struct RenameRes;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteReq {
    r#type: project_manager::ProjectType,
    id: String,
}

#[derive(Debug, serde::Serialize)]
struct DeleteRes;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenReq {
    r#type: project_manager::ProjectType,
    id: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
struct OpenRes(protocol::NotifierId);

#[derive(Debug, serde::Deserialize)]
#[serde(transparent)]
struct CloseReq(protocol::NotifierId);

#[derive(Debug, serde::Serialize)]
struct CloseRes;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CallOpenedReq {
    notifier_id: String,
    call_data: project_manager::ProjectCall,
}

#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
struct CallOpenedRes(project_manager::ProjectCallResult);

trait ProjectOperations: Default {
    type StoreItem: DeserializeOwned + Serialize;

    fn get_type(&self) -> project_manager::ProjectType;
}

#[derive(Debug, Default)]
struct CoreProjectOperations {}

impl ProjectOperations for CoreProjectOperations {
    type StoreItem = project_manager::CoreProject;

    fn get_type(&self) -> project_manager::ProjectType {
        project_manager::ProjectType::Core
    }
}

#[derive(Debug, Default)]
struct UiProjectOperations {}

impl ProjectOperations for UiProjectOperations {
    type StoreItem = project_manager::UiProject;

    fn get_type(&self) -> project_manager::ProjectType {
        project_manager::ProjectType::Ui
    }
}

trait UntypedProjectStore {
    fn get_project_infos(&self) -> Vec<(String, project_manager::ProjectInfo)>;
    fn get_type(&self) -> project_manager::ProjectType;
}

#[derive(Debug)]
struct ProjectStore<Operations: ProjectOperations> {
    fs_collection: FsCollection<Operations::StoreItem>,
    ops: Operations,
}

impl<Operations: ProjectOperations> ProjectStore<Operations> {
    pub fn new(store_path: PathBuf) -> Self {
        Self {
            fs_collection: FsCollection::new(store_path),
            ops: Operations::default(),
        }
    }
}

impl<Operations: ProjectOperations> UntypedProjectStore for ProjectStore<Operations> {
    fn get_project_infos(&self) -> Vec<(String, project_manager::ProjectInfo)> {
        // TODO
        Vec::new()
    }

    fn get_type(&self) -> project_manager::ProjectType {
        self.ops.get_type()
    }
}

#[derive(Debug)]
pub struct ProjectManagerConfig {
    pub core_projects_store_path: PathBuf,
    pub ui_projects_store_path: PathBuf,
}

#[derive(Debug)]
struct ProjectManager {
    list_notifiers: NotifierManager<UpdateListNotification>,
    core_project_store: ProjectStore<CoreProjectOperations>,
    ui_project_store: ProjectStore<UiProjectOperations>,
}

impl ProjectManager {
    fn get_project_store(
        &mut self,
        r#type: project_manager::ProjectType,
    ) -> &mut dyn UntypedProjectStore {
        match r#type {
            project_manager::ProjectType::Core => &mut self.core_project_store,
            project_manager::ProjectType::Ui => &mut self.ui_project_store,
        }
    }
}

impl Actor for ProjectManager {
    type Args = ProjectManagerConfig;
    type Error = HandleLookupError;

    async fn on_start(args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(Self {
            list_notifiers: NotifierManager::new("project-manager/list"),
            core_project_store: ProjectStore::new(args.core_projects_store_path),
            ui_project_store: ProjectStore::new(args.ui_projects_store_path),
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
        // TODO: close opened projects
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

        for (name, info) in self.core_project_store.get_project_infos() {
            notifier.notify(&project_manager::UpdateListNotification::Set(
                project_manager::SetListNotification {
                    r#type: project_manager::ProjectType::Core,
                    name,
                    info,
                },
            ));
        }

        for (name, info) in self.ui_project_store.get_project_infos() {
            notifier.notify(&project_manager::UpdateListNotification::Set(
                project_manager::SetListNotification {
                    r#type: project_manager::ProjectType::Ui,
                    name,
                    info,
                },
            ));
        }
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

impl message::Message<ServiceRequest<CreateNewReq>> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<CreateNewReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
    }
}

impl message::Message<ServiceRequest<DuplicateReq>> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<DuplicateReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
    }
}

impl message::Message<ServiceRequest<RenameReq>> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<RenameReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
    }
}

impl message::Message<ServiceRequest<DeleteReq>> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<DeleteReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
    }
}

impl message::Message<ServiceRequest<OpenReq>> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<OpenReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
    }
}

impl message::Message<ServiceRequest<CloseReq>> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<CloseReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
    }
}

impl message::Message<ServiceRequest<CallOpenedReq>> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<CallOpenedReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
    }
}
