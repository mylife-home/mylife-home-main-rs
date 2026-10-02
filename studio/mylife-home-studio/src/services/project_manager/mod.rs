use std::{
    collections::{HashMap, HashSet},
    format,
    path::PathBuf,
    time::Duration,
};

use common::utils::actors::{
    ActorHandle, CallError, HandleLookupError, SchedulerHandle, SpawnedActor, SpawnedActors,
};
use kameo::{message, prelude::*};
use studio_web_api::{component_model, project_manager, protocol};
use thiserror::Error;

use crate::{
    services::project_manager::{
        fs_collection::{Event, FsCollection, FsCollectionError, Kind, Origin},
        opened_projects::OpenedProjects,
    },
    web::{DispatcherBuilder, Notifier, NotifierManager, ServiceRequest, SessionEvent},
};

mod core_project;
mod fs_collection;
mod opened_projects;
mod ui_project;

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
    list_notifiers: NotifierManager<project_manager::UpdateListNotification>,
    core_project_collection: FsCollection<project_manager::CoreProject>,
    ui_project_collection: FsCollection<project_manager::UiProject>,
    opened_projects: OpenedProjects,
}

/// Error that occurs when the ProjectManager actor fails to start due to issues with actor handle lookup or scheduler setup.
#[derive(Debug, Error)]
pub enum ProjectManagerActorError {
    #[error("Failed to lookup actor handle: {0}")]
    HandleLookupError(#[from] HandleLookupError),
    #[error("Failed to set interval: {0}")]
    SchedulerError(#[from] CallError),
    #[error("Failed to access data: {0}")]
    DataAccessError(#[from] FsCollectionError),
    #[error("Failed to process data: {0}")]
    DataProcessingError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Project not found: {0}")]
    ProjectNotFound(String),
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
            opened_projects: OpenedProjects::new(),
        })
    }
}

impl ProjectManager {
    fn compute_core_project_info(
        &self,
        id: &str,
    ) -> Result<project_manager::ProjectInfo, ProjectManagerActorError> {
        let project = self.core_project_collection.get(id)?;

        let mut instances = HashSet::new();

        for plugin in project.plugins.values() {
            instances.insert(plugin.instance_name.as_str());
        }

        let mut info = project_manager::CoreProjectInfo {
            instances_count: instances.len(),
            plugins_count: project.plugins.len(),
            templates_count: project.templates.len(),
            components_counts: HashMap::from([
                (component_model::PluginUsage::Sensor, 0),
                (component_model::PluginUsage::Actuator, 0),
                (component_model::PluginUsage::Logic, 0),
                (component_model::PluginUsage::Ui, 0),
            ]),
            bindings_count: 0,
        };

        let mut process_components_bindings =
            |components: &HashMap<String, project_manager::CoreComponentData>,
             bindings: &HashMap<String, project_manager::CoreBindingData>|
             -> Result<(), ProjectManagerActorError> {
                info.bindings_count += bindings.len();

                for component in components.values() {
                    let usage = match &component.definition.r#type {
                        project_manager::CoreComponentDefinitionType::Plugin => {
                            let Some(plugin) = project.plugins.get(&component.definition.id) else {
                                return Err(ProjectManagerActorError::DataProcessingError(
                                    format!(
                                        "Plugin '{}' not found in project",
                                        &component.definition.id
                                    ),
                                ));
                            };

                            plugin.usage
                        }
                        project_manager::CoreComponentDefinitionType::Template => {
                            component_model::PluginUsage::Logic
                        }
                    };

                    *info.components_counts.entry(usage).or_insert(0) += 1;
                }

                Ok(())
            };

        process_components_bindings(&project.components, &project.bindings)?;

        for template in project.templates.values() {
            process_components_bindings(&template.components, &template.bindings)?;
        }

        Ok(project_manager::ProjectInfo(serde_json::to_value(&info)?))
    }

    fn compute_ui_project_info(
        &self,
        id: &str,
    ) -> Result<project_manager::ProjectInfo, ProjectManagerActorError> {
        let project = self.ui_project_collection.get(id)?;

        let resource_binary_length = |resource: &project_manager::UiResourceData| -> usize {
            // base64 length = 4 chars represents 3 binary bytes
            (resource.data.len() * 3) / 4
        };

        let info = project_manager::UiProjectInfo {
            windows_count: project.windows.len(),
            resources_count: project.resources.len(),
            resources_size: project
                .resources
                .iter()
                .map(|(_, res)| resource_binary_length(res))
                .sum(),
            styles_count: project.styles.len(),
            components_count: project.components.len(),
        };

        Ok(project_manager::ProjectInfo(serde_json::to_value(&info)?))
    }

    fn compute_project_info(
        &self,
        ty: project_manager::ProjectType,
        id: &str,
    ) -> Result<project_manager::ProjectInfo, ProjectManagerActorError> {
        match ty {
            project_manager::ProjectType::Core => self.compute_core_project_info(id),
            project_manager::ProjectType::Ui => self.compute_ui_project_info(id),
        }
    }

    fn translate_event(
        &self,
        ty: project_manager::ProjectType,
        event: &Event,
    ) -> Result<project_manager::UpdateListNotification, ProjectManagerActorError> {
        let notification = match &event.kind {
            Kind::Created | Kind::Updated => {
                let info = self.compute_project_info(ty, &event.id)?;

                project_manager::UpdateListNotification::Set(project_manager::SetListNotification {
                    r#type: ty,
                    name: event.id.clone(),
                    info,
                })
            }
            Kind::Deleted => project_manager::UpdateListNotification::Clear(
                project_manager::ClearListNotification {
                    r#type: ty,
                    name: event.id.clone(),
                },
            ),
            Kind::Renamed { new_id } => project_manager::UpdateListNotification::Rename(
                project_manager::RenameListNotification {
                    r#type: ty,
                    name: event.id.clone(),
                    new_name: new_id.clone(),
                },
            ),
        };

        Ok(notification)
    }

    fn process_events(&mut self, event_collector: Vec<Event>, ty: project_manager::ProjectType) {
        for event in event_collector {
            self.process_event(ty, &event);
        }

        // Services.instance.git.notifyFileUpdate();
    }

    fn process_event(&mut self, ty: project_manager::ProjectType, event: &Event) {
        let notification = match self.translate_event(ty, &event) {
            Ok(notification) => notification,
            Err(_) => {
                tracing::error!("Failed to translate event: {:?}", event);
                return;
            }
        };

        self.list_notifiers.notify_all(&notification);

        if let Kind::Updated = event.kind
            && event.origin == Origin::External
        {
            self.process_external_update(ty, &event.id);
        }
    }

    fn process_external_update(&mut self, ty: project_manager::ProjectType, id: &str) {
        tracing::info!(?ty, id, "Processing external project update");

        match ty {
            project_manager::ProjectType::Core => {
                let project = match self.core_project_collection.get(id) {
                    Ok(project) => project,
                    Err(error) => {
                        tracing::error!(%error, id, "Failed to get core project from collection");
                        return;
                    }
                };

                self.opened_projects
                    .reload_project(id, opened_projects::TypedProjectData::Core(project.clone()));
            }
            project_manager::ProjectType::Ui => {
                let project = match self.ui_project_collection.get(id) {
                    Ok(project) => project,
                    Err(error) => {
                        tracing::error!(%error, id, "Failed to get UI project from collection");
                        return;
                    }
                };

                self.opened_projects
                    .reload_project(id, opened_projects::TypedProjectData::Ui(project.clone()));
            }
        }
    }

    fn emit_initial(
        &self,
        notifier: &Notifier<project_manager::UpdateListNotification>,
        ty: project_manager::ProjectType,
        id: &str,
    ) {
        let info = match self.compute_project_info(ty, id) {
            Ok(info) => info,
            Err(e) => {
                tracing::error!("Failed to compute project info for id {}: {:?}", id, e);
                return;
            }
        };

        let notification =
            project_manager::UpdateListNotification::Set(project_manager::SetListNotification {
                r#type: ty,
                name: id.to_owned(),
                info,
            });

        notifier.notify(&notification);
    }

    async fn create_new_project(
        &mut self,
        event_collector: &mut Vec<Event>,
        ty: project_manager::ProjectType,
        id: &str,
    ) -> Result<(), ProjectManagerActorError> {
        match ty {
            project_manager::ProjectType::Core => {
                // Implementation for creating a new core project goes here
                let project = project_manager::CoreProject {
                    components: HashMap::new(),
                    plugins: HashMap::new(),
                    bindings: HashMap::new(),
                    templates: HashMap::new(),
                };

                Ok(self
                    .core_project_collection
                    .create(event_collector, id, project)
                    .await?)
            }
            project_manager::ProjectType::Ui => {
                let project = project_manager::UiProject {
                    resources: HashMap::new(),
                    styles: HashMap::new(),
                    windows: HashMap::new(),
                    templates: HashMap::new(),
                    default_window: project_manager::UiDefaultWindowData(HashMap::from_iter([
                        ("desktop".to_string(), None),
                        ("mobile".to_string(), None),
                    ])),
                    components: HashMap::new(),
                    plugins: HashMap::new(),
                };

                Ok(self
                    .ui_project_collection
                    .create(event_collector, id, project)
                    .await?)
            }
        }
    }

    async fn duplicate_project(
        &mut self,
        event_collector: &mut Vec<Event>,
        ty: project_manager::ProjectType,
        id: &str,
        new_id: &str,
    ) -> Result<(), ProjectManagerActorError> {
        match ty {
            project_manager::ProjectType::Core => {
                let source = self.core_project_collection.get(id)?;
                let value = serde_json::to_value(source)?;
                let duplicate = serde_json::from_value(value)?;

                Ok(self
                    .core_project_collection
                    .create(event_collector, new_id, duplicate)
                    .await?)
            }
            project_manager::ProjectType::Ui => {
                let source = self.ui_project_collection.get(id)?;
                let value = serde_json::to_value(source)?;
                let duplicate = serde_json::from_value(value)?;

                Ok(self
                    .ui_project_collection
                    .create(event_collector, new_id, duplicate)
                    .await?)
            }
        }
    }

    async fn handle_project_update(
        &mut self,
        project_data: Option<(String, opened_projects::TypedProjectData)>,
    ) {
        let Some((id, project_data)) = project_data else {
            return;
        };

        match project_data {
            opened_projects::TypedProjectData::Core(core_project) => {
                let mut event_collector = Vec::new();

                if let Err(error) = self
                    .core_project_collection
                    .update(&mut event_collector, &id, core_project)
                    .await
                {
                    tracing::error!(%error, id, "Failed to update core project");
                }

                self.process_events(event_collector, project_manager::ProjectType::Core);
            }
            opened_projects::TypedProjectData::Ui(ui_project) => {
                let mut event_collector = Vec::new();

                if let Err(error) = self
                    .ui_project_collection
                    .update(&mut event_collector, &id, ui_project)
                    .await
                {
                    tracing::error!(%error, id, "Failed to update UI project");
                }

                self.process_events(event_collector, project_manager::ProjectType::Ui);
            }
        }
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
        let mut core_event_collector = Vec::new();
        let mut ui_event_collector = Vec::new();

        self.core_project_collection
            .refresh(&mut core_event_collector)
            .await;
        self.ui_project_collection
            .refresh(&mut ui_event_collector)
            .await;

        self.process_events(core_event_collector, project_manager::ProjectType::Core);
        self.process_events(ui_event_collector, project_manager::ProjectType::Ui);
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
        self.opened_projects.session_event(&msg);
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
        let notifier = self
            .list_notifiers
            .create_notifier(call.session().clone())
            .clone();

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
            .remove_notifier(call.session(), notifier_id.notifier_id.as_str());

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
        let request = call.request();
        let ty = request.r#type;
        let id = request.id.clone();

        let mut event_collector = Vec::new();
        let res = self.create_new_project(&mut event_collector, ty, &id).await;

        call.reply_result(res.map(|_| CreateNewRes {
            r#type: ty,
            created_id: id,
        }));

        self.process_events(event_collector, ty);
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
        let request = call.request();
        let ty = request.r#type;
        let new_id = request.new_id.clone();

        let mut event_collector = Vec::new();
        let res = self
            .duplicate_project(&mut event_collector, ty, &request.id, &new_id)
            .await;

        call.reply_result(res.map(|_| DuplicateRes {
            r#type: ty,
            created_id: new_id,
        }));

        self.process_events(event_collector, ty);
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
        let request = call.request();
        let ty = request.r#type;

        let mut event_collector = Vec::new();
        let res = match ty {
            project_manager::ProjectType::Core => {
                self.core_project_collection
                    .rename(&mut event_collector, &request.id, &request.new_id)
                    .await
            }
            project_manager::ProjectType::Ui => {
                self.ui_project_collection
                    .rename(&mut event_collector, &request.id, &request.new_id)
                    .await
            }
        };

        call.reply_result(res.map(|_| RenameRes));

        self.process_events(event_collector, ty);
    }
}

impl message::Message<ServiceRequest<DeleteReq>> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<DeleteReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call: crate::web::ServiceCall<DeleteReq> = request.into_call();
        let request = call.request();
        let ty = request.r#type;

        // TODO: fail if opened?

        let mut event_collector = Vec::new();
        let res = match ty {
            project_manager::ProjectType::Core => {
                self.core_project_collection
                    .delete(&mut event_collector, &request.id)
                    .await
            }
            project_manager::ProjectType::Ui => {
                self.ui_project_collection
                    .delete(&mut event_collector, &request.id)
                    .await
            }
        };

        call.reply_result(res.map(|_| DeleteRes));

        self.process_events(event_collector, ty);
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
        let request = call.request();

        let project_data = match request.r#type {
            project_manager::ProjectType::Core => {
                let Some(core_project) = self.core_project_collection.get(&request.id).ok() else {
                    let err = ProjectManagerActorError::ProjectNotFound(request.id.clone());
                    call.reply_error(err);
                    return;
                };

                opened_projects::TypedProjectData::Core(core_project.clone())
            }
            project_manager::ProjectType::Ui => {
                let Some(ui_project) = self.ui_project_collection.get(&request.id).ok() else {
                    let err = ProjectManagerActorError::ProjectNotFound(request.id.clone());
                    call.reply_error(err);
                    return;
                };

                opened_projects::TypedProjectData::Ui(ui_project.clone())
            }
        };

        let mut notifications_emitter =
            match self
                .opened_projects
                .open_project(call.session(), project_data, &request.id)
            {
                Ok((notifier_id, notifications_emitter)) => {
                    call.reply_ok(OpenRes(notifier_id));

                    notifications_emitter
                }
                Err(err) => {
                    call.reply_error(err);
                    return;
                }
            };

        notifications_emitter.emit_notifications();
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
        let request = call.request();

        let res = self
            .opened_projects
            .close_project(call.session(), request.0.clone());

        call.reply_result(res.map(|_| CloseRes));
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
        let request = call.request();
        let notifier_id = protocol::NotifierId {
            notifier_id: request.notifier_id.clone(),
        };

        let (mut notifications_emitter, project_data) = match self
            .opened_projects
            .call_project(call.session(), notifier_id, request.call_data.clone())
            .await
        {
            Ok((data, notifications_emitter, project_data)) => {
                call.reply_ok(CallOpenedRes(data));
                (notifications_emitter, project_data)
            }
            Err(err) => {
                call.reply_error(err);
                return;
            }
        };

        notifications_emitter.emit_notifications();

        self.handle_project_update(project_data).await;
    }
}
