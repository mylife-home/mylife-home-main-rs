use std::{collections::HashMap, path::PathBuf, time::Duration};

use common::utils::actors::{
    ActorHandle, CallError, HandleLookupError, SchedulerHandle, SpawnedActor, SpawnedActors,
};
use kameo::{message, prelude::*};
use studio_web_api::{
    component_model, project_manager::{self, UpdateListNotification}, protocol,
};
use thiserror::Error;

use crate::{
    services::project_manager::fs_collection::{Event, FsCollection, FsCollectionError, Kind}, web::{DispatcherBuilder, Notifier, NotifierManager, ServiceRequest, SessionEvent},
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

#[derive(Debug)]
pub struct ProjectManagerConfig {
    pub core_projects_store_path: PathBuf,
    pub ui_projects_store_path: PathBuf,
}

#[derive(Debug)]
struct ProjectManager {
    list_notifiers: NotifierManager<UpdateListNotification>,
    core_project_collection: FsCollection<project_manager::CoreProject>,
    ui_project_collection: FsCollection<project_manager::UiProject>,
}

/// Error that occurs when the ProjectManager actor fails to start due to issues with actor handle lookup or scheduler setup.
#[derive(Debug, Error)]
pub enum ProjectManagerActorError {
    #[error("Failed to lookup actor handle: {0}")]
    HandleLookupError(#[from] HandleLookupError),
    #[error("Failed to set interval: {0}")]
    SchedulerError(#[from] CallError),
}

impl Actor for ProjectManager {
    type Args = ProjectManagerConfig;
    type Error = ProjectManagerActorError;

    async fn on_start(args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let scheduler = SchedulerHandle::new()?;

        scheduler
            .set_interval(actor_ref.downgrade(), Duration::from_millis(200), Refresh)
            .await?;

        Ok(Self {
            list_notifiers: NotifierManager::new("project-manager/list"),
            core_project_collection: FsCollection::new(args.core_projects_store_path),
            ui_project_collection: FsCollection::new(args.ui_projects_store_path),
        })
    }
}

impl ProjectManager {
    fn compute_core_project_info(
        &self,
        id: &str,
    ) -> Result<project_manager::ProjectInfo, FsCollectionError> {
        let core_project = self.core_project_collection.get(id)?;

        let mut info = project_manager::CoreProjectInfo {
            instances_count: 0,
            plugins_count: 0,
            templates_count: 0,
            components_counts: HashMap::from([
                (component_model::PluginUsage::Sensor, 0),
                (component_model::PluginUsage::Actuator, 0),
                (component_model::PluginUsage::Logic, 0),
                (component_model::PluginUsage::Ui, 0),
            ]),
            bindings_count: 0,
        };

        Ok(project_manager::ProjectInfo(serde_json::to_value(&info)?))
    }

    fn compute_ui_project_info(
        &self,
        id: &str,
    ) -> Result<project_manager::ProjectInfo, FsCollectionError> {
        let ui_project = self.ui_project_collection.get(id)?;

        let mut info = project_manager::UiProjectInfo {
            windows_count: 0,
            resources_count: 0,
            resources_size: 0,
            styles_count: 0,
            components_count: 0,
        };

        Ok(project_manager::ProjectInfo(serde_json::to_value(&info)?))
    }

    fn compute_project_info(
        &self,
        ty: project_manager::ProjectType,
        id: &str,
    ) -> Result<project_manager::ProjectInfo, FsCollectionError> {
        match ty {
            project_manager::ProjectType::Core => self.compute_core_project_info(id),
            project_manager::ProjectType::Ui => self.compute_ui_project_info(id),
        }
    }

    fn translate_event(
        &self,
        ty: project_manager::ProjectType,
        event: &Event,
    ) -> Result<UpdateListNotification, FsCollectionError> {
        let notification = match &event.kind {
            Kind::Created | Kind::Updated => {
                let info = self.compute_project_info(ty, &event.id)?;

                UpdateListNotification::Set(project_manager::SetListNotification {
                    r#type: ty,
                    name: event.id.clone(),
                    info,
                })
            }
            Kind::Deleted => {
                UpdateListNotification::Clear(project_manager::ClearListNotification {
                    r#type: ty,
                    name: event.id.clone(),
                })
            }
            Kind::Renamed { new_id } => {
                UpdateListNotification::Rename(project_manager::RenameListNotification {
                    r#type: ty,
                    name: event.id.clone(),
                    new_name: new_id.clone(),
                })
            }
        };

        Ok(notification)
    }

    fn emit_event(&self, ty: project_manager::ProjectType, event: &Event) {
        let notification = match self.translate_event(ty, &event) {
            Ok(notification) => notification,
            Err(_) => {
                tracing::error!("Failed to translate event: {:?}", event);
                return;
            }
        };

        self.list_notifiers.notify_all(&notification);
    }

    fn emit_initial(&self, notifier: &Notifier<UpdateListNotification>, ty: project_manager::ProjectType, id: &str) {
        let info = match self.compute_project_info(ty, id) {
            Ok(info) => info,
            Err(e) => {
                tracing::error!("Failed to compute project info for id {}: {:?}", id, e);
                return;
            }
        };

        let notification = UpdateListNotification::Set(project_manager::SetListNotification {
            r#type: ty,
            name: id.to_owned(),
            info,
        });

        notifier.notify(&notification);
    }
}

#[derive(Debug, Clone)]
struct Refresh;

impl message::Message<Refresh> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: Refresh,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let (_, events) = self.core_project_collection.refresh().await.into();
        for event in events {
            self.emit_event(project_manager::ProjectType::Core, &event);
        }

        let (_, events) = self.ui_project_collection.refresh().await.into();
        for event in events {
            self.emit_event(project_manager::ProjectType::Ui, &event);
        }
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
        let notifier = self.list_notifiers.create_notifier(call.session().clone()).clone();

        call.reply_ok(StartNotifyListRes(protocol::NotifierId {
            notifier_id: notifier.notifier_id().into(),
        }));

        for id in self.core_project_collection.ids() {
            self.emit_initial(&notifier, project_manager::ProjectType::Core, id);
        }

        for id in self.ui_project_collection.ids() {
            self.emit_initial(&notifier, project_manager::ProjectType::Ui, id);
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
