use std::{
    collections::{HashMap, HashSet},
    format,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use common::utils::actors::{
    ActorHandle, CallError, HandleLookupError, SpawnedActor, SpawnedActors,
};
use kameo::{message, prelude::*};
use studio_web_api::{component_model, project_manager, protocol};
use thiserror::Error;

use crate::{
    services::project_manager::{
        core_project::{CoreProjectHandle, new_core_project_data},
        ui_project::{UiProjectHandle, new_ui_project_data},
    },
    utils::fs_collection::{self, FsCollectionError, FsCollectionHandle},
    web::{
        DispatcherBuilder, Notifier, NotifierManager, ServiceRequest, SessionEvent, SessionHandle,
    },
};

mod core_project;
mod ui_project;

const PROJECT_MANAGER_NAME: &str = "project-manager";

const CORE_PROJECTS_FS_COLLECTION: &str = "core-projects";
const UI_PROJECTS_FS_COLLECTION: &str = "ui-projects";

#[derive(Debug)]
pub struct ProjectManagerConfig {
    pub core_projects_store_path: PathBuf,
    pub ui_projects_store_path: PathBuf,
}

pub async fn init(
    actors: &mut SpawnedActors,
    dispatcher: &mut DispatcherBuilder,
    config: ProjectManagerConfig,
) {
    fs_collection::init_collection::<project_manager::CoreProject>(
        actors,
        CORE_PROJECTS_FS_COLLECTION,
        config.core_projects_store_path,
    )
    .await;
    fs_collection::init_collection::<project_manager::UiProject>(
        actors,
        UI_PROJECTS_FS_COLLECTION,
        config.ui_projects_store_path,
    )
    .await;

    let (project_manager, _) = SpawnedActor::start::<ProjectManager>(()).await;

    project_manager.register(PROJECT_MANAGER_NAME);

    actors.add(project_manager);

    fs_collection::start_collection::<project_manager::CoreProject>(CORE_PROJECTS_FS_COLLECTION)
        .await;
    fs_collection::start_collection::<project_manager::UiProject>(UI_PROJECTS_FS_COLLECTION).await;

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
struct ProjectManager {
    list_notifiers: NotifierManager<project_manager::UpdateListNotification>,

    core_project_collection: FsCollectionHandle<project_manager::CoreProject>,
    ui_project_collection: FsCollectionHandle<project_manager::UiProject>,

    opened_projects_notifiers: HashMap<String, Arc<OpenedProjectEntry>>,
    opened_projects_names: HashMap<(project_manager::ProjectType, String), Arc<OpenedProjectEntry>>,
}

#[derive(Debug)]
struct OpenedProjectEntry {
    name: Mutex<String>, // Allow immutable rename
    open_count: AtomicUsize,
    handle: OpenedProjectHandle,
}

impl OpenedProjectEntry {
    pub async fn create_core(
        name: String,
        fs_collection: FsCollectionHandle<project_manager::CoreProject>,
    ) -> Result<Arc<Self>, ProjectManagerActorError> {
        tracing::debug!(name, "creating opened project");

        let handle = OpenedProjectHandle::Core(
            CoreProjectHandle::create(name.clone(), fs_collection).await?,
        );

        Ok(Arc::new(Self {
            name: Mutex::new(name),
            open_count: AtomicUsize::new(0),
            handle,
        }))
    }

    pub async fn create_ui(
        name: String,
        fs_collection: FsCollectionHandle<project_manager::UiProject>,
    ) -> Result<Arc<Self>, ProjectManagerActorError> {
        tracing::debug!(name, "creating opened project");

        let handle =
            OpenedProjectHandle::Ui(UiProjectHandle::create(name.clone(), fs_collection).await?);

        Ok(Arc::new(Self {
            name: Mutex::new(name),
            open_count: AtomicUsize::new(0),
            handle,
        }))
    }

    pub async fn session_event(&self, event: SessionEvent) -> Vec<String> {
        match &self.handle {
            OpenedProjectHandle::Core(handle) => handle.session_event(event).await,
            OpenedProjectHandle::Ui(handle) => handle.session_event(event).await,
        }
    }

    pub async fn terminate(&self) {
        tracing::debug!(name = self.name(), "terminating opened project");

        match &self.handle {
            OpenedProjectHandle::Core(handle) => handle.terminate().await,
            OpenedProjectHandle::Ui(handle) => handle.terminate().await,
        }
    }

    pub async fn open(
        &self,
        session: SessionHandle,
    ) -> Result<(String, InitialEmitter), ProjectManagerActorError> {
        match &self.handle {
            OpenedProjectHandle::Core(handle) => handle.open(session).await,
            OpenedProjectHandle::Ui(handle) => handle.open(session).await,
        }
    }

    pub async fn close(
        &self,
        session: SessionHandle,
        notifier_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        match &self.handle {
            OpenedProjectHandle::Core(handle) => handle.close(session, notifier_id).await,
            OpenedProjectHandle::Ui(handle) => handle.close(session, notifier_id).await,
        }
    }

    pub async fn call(
        &self,
        session: SessionHandle,
        notifier_id: String,
        call_data: project_manager::ProjectCall,
    ) -> Result<project_manager::ProjectCallResult, ProjectManagerActorError> {
        match &self.handle {
            OpenedProjectHandle::Core(handle) => handle.call(session, notifier_id, call_data).await,
            OpenedProjectHandle::Ui(handle) => handle.call(session, notifier_id, call_data).await,
        }
    }

    pub fn rename(&self, new_name: String) {
        let mut name = self.name.lock().expect("Failed to lock name mutex");
        *name = new_name;
    }

    pub fn name(&self) -> String {
        self.name.lock().expect("Failed to lock name mutex").clone()
    }

    pub fn project_type(&self) -> project_manager::ProjectType {
        match &self.handle {
            OpenedProjectHandle::Core(_) => project_manager::ProjectType::Core,
            OpenedProjectHandle::Ui(_) => project_manager::ProjectType::Ui,
        }
    }

    /// Increments the open count for the project.
    pub fn inc_open_count(&self) {
        self.open_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrements the open count and returns true if the project is still opened by any session.
    pub fn dec_open_count(&self) -> bool {
        self.open_count.fetch_sub(1, Ordering::SeqCst) > 1
    }

    pub fn used(&self) -> bool {
        self.open_count.load(Ordering::SeqCst) > 0
    }
}

#[derive(Debug, Clone)]
enum OpenedProjectHandle {
    Core(CoreProjectHandle),
    Ui(UiProjectHandle),
}

/// Represents an emitter that initially sends a set of notifications for an opened project.
#[derive(Debug)]
struct InitialEmitter {
    notifier: Notifier<project_manager::UpdateProjectNotification>,
    notifications: Vec<project_manager::UpdateProjectNotification>,
}

impl InitialEmitter {
    /// Creates a new instance of `InitialEmitter` with the specified notifier and initial notifications.
    pub fn new(
        notifier: Notifier<project_manager::UpdateProjectNotification>,
        notifications: Vec<project_manager::UpdateProjectNotification>,
    ) -> Self {
        Self {
            notifier,
            notifications,
        }
    }

    pub fn emit_notifications(self) {
        for notification in self.notifications {
            self.notifier.notify(&notification);
        }
    }
}

/// Error that occurs when the ProjectManager actor fails to start due to issues with actor handle lookup or scheduler setup.
#[derive(Debug, Error)]
pub enum ProjectManagerActorError {
    #[error("Failed to lookup actor handle: {0}")]
    HandleLookupError(#[from] HandleLookupError),
    #[error("Failed to access data: {0}")]
    DataAccessError(#[from] CallError<FsCollectionError>),
    #[error("Failed to process data: {0}")]
    DataProcessingError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Project not found: {0}")]
    ProjectNotFound(String),
    #[error("Opened project error: {0}")]
    OpenedProjectError(Box<CallError<ProjectManagerActorError>>),
    #[error("Notifier not found: {0}")]
    NotifierNotFound(String),
}

impl Actor for ProjectManager {
    type Args = ();
    type Error = ProjectManagerActorError;

    async fn on_start(_args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let core_project_collection = FsCollectionHandle::new(CORE_PROJECTS_FS_COLLECTION)?;
        let ui_project_collection = FsCollectionHandle::new(UI_PROJECTS_FS_COLLECTION)?;

        core_project_collection
            .on_update()
            .subscribe(actor_ref.clone());
        ui_project_collection
            .on_update()
            .subscribe(actor_ref.clone());

        Ok(Self {
            list_notifiers: NotifierManager::new("project-manager/list"),
            core_project_collection,
            ui_project_collection,
            opened_projects_notifiers: HashMap::new(),
            opened_projects_names: HashMap::new(),
        })
    }
}

impl message::Message<fs_collection::Update<project_manager::CoreProject>> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: fs_collection::Update<project_manager::CoreProject>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let fs_collection::Kind::Renamed { new_id } = &msg.kind {
            self.rename(
                project_manager::ProjectType::Core,
                msg.id.clone(),
                new_id.clone(),
            );
        }

        let notification = match msg.kind {
            fs_collection::Kind::Created { value } | fs_collection::Kind::Updated { value } => {
                let info = match self.compute_core_project_info(&value) {
                    Ok(info) => info,
                    Err(err) => {
                        tracing::error!(%err, id = %msg.id, "failed to compute core project info");
                        return;
                    }
                };

                project_manager::UpdateListNotification::Set(project_manager::SetListNotification {
                    r#type: project_manager::ProjectType::Core,
                    name: msg.id.clone(),
                    info,
                })
            }
            fs_collection::Kind::Renamed { new_id } => {
                project_manager::UpdateListNotification::Rename(
                    project_manager::RenameListNotification {
                        r#type: project_manager::ProjectType::Core,
                        name: msg.id.clone(),
                        new_name: new_id.clone(),
                    },
                )
            }
            fs_collection::Kind::Deleted => project_manager::UpdateListNotification::Clear(
                project_manager::ClearListNotification {
                    r#type: project_manager::ProjectType::Core,
                    name: msg.id.clone(),
                },
            ),
        };

        self.list_notifiers.notify_all(&notification);
    }
}

impl message::Message<fs_collection::Update<project_manager::UiProject>> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: fs_collection::Update<project_manager::UiProject>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let fs_collection::Kind::Renamed { new_id } = &msg.kind {
            self.rename(
                project_manager::ProjectType::Ui,
                msg.id.clone(),
                new_id.clone(),
            );
        }

        let notification = match msg.kind {
            fs_collection::Kind::Created { value } | fs_collection::Kind::Updated { value } => {
                let info = match self.compute_ui_project_info(&value) {
                    Ok(info) => info,
                    Err(err) => {
                        tracing::error!(%err, id = %msg.id, "failed to compute UI project info");
                        return;
                    }
                };

                project_manager::UpdateListNotification::Set(project_manager::SetListNotification {
                    r#type: project_manager::ProjectType::Ui,
                    name: msg.id.clone(),
                    info,
                })
            }
            fs_collection::Kind::Renamed { new_id } => {
                project_manager::UpdateListNotification::Rename(
                    project_manager::RenameListNotification {
                        r#type: project_manager::ProjectType::Ui,
                        name: msg.id.clone(),
                        new_name: new_id.clone(),
                    },
                )
            }
            fs_collection::Kind::Deleted => project_manager::UpdateListNotification::Clear(
                project_manager::ClearListNotification {
                    r#type: project_manager::ProjectType::Ui,
                    name: msg.id.clone(),
                },
            ),
        };

        self.list_notifiers.notify_all(&notification);
    }
}

impl ProjectManager {
    fn rename(&mut self, ty: project_manager::ProjectType, old_id: String, new_id: String) {
        if let Some(project) = self.opened_projects_names.remove(&(ty, old_id.clone())) {
            // Only rename the entry, the opened project itself will be renamed through its actor mailbox
            project.rename(new_id.clone());
            self.opened_projects_names.insert((ty, new_id), project);
        }
    }

    fn compute_core_project_info(
        &self,
        project: &project_manager::CoreProject,
    ) -> Result<project_manager::ProjectInfo, ProjectManagerActorError> {
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
        project: &project_manager::UiProject,
    ) -> Result<project_manager::ProjectInfo, ProjectManagerActorError> {
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
}

impl message::Message<SessionEvent> for ProjectManager {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.list_notifiers.session_event(&msg);

        // dispatch to opened projects (they cannot register session events directly)
        let futures = self
            .opened_projects_names
            .values()
            .map(|project| project.session_event(msg.clone()));
        let removed_notifiers = futures::future::join_all(futures)
            .await
            .into_iter()
            .flatten();

        for removed_notifier in removed_notifiers {
            self.close_project(removed_notifier).await;
        }
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

        self.emit_initial(&notifier).await;
    }
}

impl ProjectManager {
    async fn emit_initial(&self, notifier: &Notifier<project_manager::UpdateListNotification>) {
        let projects = match self.core_project_collection.get_all().await {
            Ok(projects) => projects,
            Err(error) => {
                tracing::error!(%error, "failed to get all core projects");
                return;
            }
        };

        for (id, project) in projects {
            let info = match self.compute_core_project_info(&project) {
                Ok(info) => info,
                Err(error) => {
                    tracing::error!(%error, "failed to compute core project info");
                    continue;
                }
            };

            let notification = project_manager::UpdateListNotification::Set(
                project_manager::SetListNotification {
                    r#type: project_manager::ProjectType::Core,
                    name: id.clone(),
                    info,
                },
            );

            notifier.notify(&notification);
        }

        let projects = match self.ui_project_collection.get_all().await {
            Ok(projects) => projects,
            Err(error) => {
                tracing::error!(%error, "failed to get all UI projects");
                return;
            }
        };

        for (id, project) in projects {
            let info = match self.compute_ui_project_info(&project) {
                Ok(info) => info,
                Err(error) => {
                    tracing::error!(%error, "failed to compute UI project info");
                    continue;
                }
            };

            let notification = project_manager::UpdateListNotification::Set(
                project_manager::SetListNotification {
                    r#type: project_manager::ProjectType::Ui,
                    name: id.clone(),
                    info,
                },
            );

            notifier.notify(&notification);
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

        if let Err(error) = self.create_new_project(ty, id.clone()).await {
            call.reply_error(error);
            return;
        }

        tracing::info!(r#type = ?ty, id, "created new project");

        call.reply_ok(CreateNewRes {
            r#type: ty,
            created_id: id,
        });
    }
}

impl ProjectManager {
    async fn create_new_project(
        &mut self,
        ty: project_manager::ProjectType,
        id: String,
    ) -> Result<(), ProjectManagerActorError> {
        match ty {
            project_manager::ProjectType::Core => Ok(self
                .core_project_collection
                .create(id, new_core_project_data())
                .await?),
            project_manager::ProjectType::Ui => Ok(self
                .ui_project_collection
                .create(id, new_ui_project_data())
                .await?),
        }
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
        let id = request.id.clone();
        let new_id = request.new_id.clone();

        if let Err(error) = self.duplicate_project(ty, id.clone(), new_id.clone()).await {
            call.reply_error(error);
            return;
        }

        tracing::info!(r#type = ?ty, id, new_id, "duplicated project");

        call.reply_ok(DuplicateRes {
            r#type: ty,
            created_id: new_id,
        });
    }
}

impl ProjectManager {
    async fn duplicate_project(
        &mut self,
        ty: project_manager::ProjectType,
        id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        match ty {
            project_manager::ProjectType::Core => {
                let source = self.core_project_collection.get(id).await?;
                // Don't clone the Arc itself, but the underlying data
                let duplicate = Arc::new(source.as_ref().clone());

                Ok(self
                    .core_project_collection
                    .create(new_id, duplicate)
                    .await?)
            }
            project_manager::ProjectType::Ui => {
                let source = self.ui_project_collection.get(id).await?;
                // Don't clone the Arc itself, but the underlying data
                let duplicate = Arc::new(source.as_ref().clone());

                Ok(self.ui_project_collection.create(new_id, duplicate).await?)
            }
        }
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
        let id = request.id.clone();
        let new_id = request.new_id.clone();

        match ty {
            project_manager::ProjectType::Core => {
                if let Err(error) = self
                    .core_project_collection
                    .rename(id.clone(), new_id.clone())
                    .await
                {
                    call.reply_error(error);
                    return;
                }
            }
            project_manager::ProjectType::Ui => {
                if let Err(error) = self
                    .ui_project_collection
                    .rename(id.clone(), new_id.clone())
                    .await
                {
                    call.reply_error(error);
                    return;
                }
            }
        };

        tracing::info!(r#type = ?ty, id, new_id, "renamed project");

        call.reply_ok(RenameRes);
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
        let id = request.id.clone();

        // TODO: fail if opened?

        match ty {
            project_manager::ProjectType::Core => {
                if let Err(error) = self
                    .core_project_collection
                    .delete(request.id.clone())
                    .await
                {
                    call.reply_error(error);
                    return;
                }
            }
            project_manager::ProjectType::Ui => {
                if let Err(error) = self.ui_project_collection.delete(request.id.clone()).await {
                    call.reply_error(error);
                    return;
                }
            }
        }

        tracing::info!(r#type = ?ty, id, "deleted project");

        call.reply_ok(DeleteRes);
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

        let project = match self
            .get_or_open_project(request.r#type, request.id.clone())
            .await
        {
            Ok(project) => project,
            Err(err) => {
                call.reply_error(err);
                return;
            }
        };

        let (notifier_id, initial_emitter) = match project.open(call.session().clone()).await {
            Ok((notifier_id, initial_emitter)) => (notifier_id, initial_emitter),
            Err(err) => {
                // Need to cleanup if just opened
                if !project.used() {
                    project.terminate().await;
                    self.opened_projects_names
                        .remove(&(project.project_type(), project.name().to_owned()));
                }

                call.reply_error(err);
                return;
            }
        };

        self.opened_projects_notifiers
            .insert(notifier_id.clone(), project.clone());
        project.inc_open_count();
        tracing::debug!(notifier_id, name = project.name(), "opened project");

        call.reply_ok(OpenRes(protocol::NotifierId { notifier_id }));

        initial_emitter.emit_notifications();
    }
}

impl ProjectManager {
    async fn get_or_open_project(
        &mut self,
        project_type: project_manager::ProjectType,
        id: String,
    ) -> Result<Arc<OpenedProjectEntry>, ProjectManagerActorError> {
        let key = (project_type, id.clone());

        let project = match self.opened_projects_names.get(&key) {
            Some(project) => project.clone(),
            None => {
                let project = match project_type {
                    project_manager::ProjectType::Core => {
                        OpenedProjectEntry::create_core(
                            id.clone(),
                            self.core_project_collection.clone(),
                        )
                        .await?
                    }
                    project_manager::ProjectType::Ui => {
                        OpenedProjectEntry::create_ui(
                            id.clone(),
                            self.ui_project_collection.clone(),
                        )
                        .await?
                    }
                };

                self.opened_projects_names.insert(key, project.clone());

                project
            }
        };

        Ok(project)
    }

    fn get_project(
        &self,
        notifier_id: &str,
    ) -> Result<&OpenedProjectEntry, ProjectManagerActorError> {
        match self.opened_projects_notifiers.get(notifier_id) {
            Some(entry) => Ok(entry),
            None => Err(ProjectManagerActorError::NotifierNotFound(
                notifier_id.to_owned(),
            )),
        }
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
        let notifier_id = request.0.notifier_id.clone();

        let project = match self.get_project(&notifier_id) {
            Ok(project) => project,
            Err(err) => {
                call.reply_error(err);
                return;
            }
        };

        match project
            .close(call.session().clone(), notifier_id.clone())
            .await
        {
            Ok(_) => {}
            Err(err) => {
                call.reply_error(err);
                return;
            }
        }

        self.close_project(notifier_id).await;

        call.reply_ok(CloseRes);
    }
}

impl ProjectManager {
    async fn close_project(&mut self, notifier_id: String) {
        let entry = match self.opened_projects_notifiers.remove(&notifier_id) {
            Some(entry) => entry,
            None => {
                tracing::error!(notifier_id, "notifier not found");
                return;
            }
        };

        let used = entry.dec_open_count();
        tracing::debug!(notifier_id, name = entry.name(), "closed project");

        if !used {
            entry.terminate().await;
            self.opened_projects_names
                .remove(&(entry.project_type(), entry.name().to_owned()));
        }
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
        let notifier_id = request.notifier_id.clone();

        let project = match self.get_project(&notifier_id) {
            Ok(project) => project,
            Err(err) => {
                call.reply_error(err);
                return;
            }
        };

        let res = match project
            .call(
                call.session().clone(),
                notifier_id,
                request.call_data.clone(),
            )
            .await
        {
            Ok(res) => res,
            Err(err) => {
                call.reply_error(err);
                return;
            }
        };

        call.reply_ok(CallOpenedRes(res));
    }
}
