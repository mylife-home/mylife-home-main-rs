mod deploy;
mod import;
pub mod model;
pub mod resolver;
mod validation;

#[cfg(test)]
mod tests;

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    mem,
    sync::Arc,
};

use common::utils::actors::{CallError, SpawnExt};
use studio_web_api::project_manager;

use serde::Serialize;

use common::components::registry::RegistryHandle;
use studio_web_api::project_manager::DeployChanges;
use studio_web_api::project_manager::core_validation::{Item, Severity};

use crate::{
    services::{
        online::OnlineInstancesHandle,
        project_manager::{
            InitialEmitter, ProjectManagerActorError, ProjectManagerHandler, ProjectModel,
        },
    },
    utils::fs_collection::{self, FsCollectionHandle, Update},
    web::{NotifierManager, SessionEvent, SessionHandle},
};

use kameo::{error::Infallible, message, prelude::*};

pub fn new_core_project_data() -> Arc<project_manager::CoreProject> {
    Arc::new(project_manager::CoreProject {
        components: HashMap::new(),
        plugins: HashMap::new(),
        bindings: HashMap::new(),
        templates: HashMap::new(),
    })
}

#[derive(Debug, Clone)]
pub struct CoreProjectHandle(ActorRef<CoreProject>);

impl CoreProjectHandle {
    pub async fn create(
        name: String,
        fs_collection: FsCollectionHandle<project_manager::CoreProject>,
    ) -> Result<Self, ProjectManagerActorError> {
        let data = fs_collection.get(name.clone()).await?;

        // A stored project the model refuses cannot be served at all, so it
        // fails here rather than on whichever call happens to touch it first.
        let model = model::Project::try_from(data.as_ref())?;

        let actor = CoreProject::spawn_unbounded(CoreProjectArgs {
            name,
            data,
            model,
            fs_collection,
        });

        // Startup cannot fail
        actor.wait_for_startup().await;

        Ok(Self(actor))
    }

    pub async fn terminate(&self) {
        if let Err(error) = self.0.stop_gracefully().await {
            tracing::error!(%error, "cannot stop project actor");
            return;
        }

        // Shutdown cannot fail
        self.0.wait_for_shutdown().await;
    }

    pub async fn open(
        &self,
        session: SessionHandle,
    ) -> Result<(String, InitialEmitter), ProjectManagerActorError> {
        let res = self.0.ask(Open { session }).try_send().await;

        Ok(res.map_err(|e| {
            ProjectManagerActorError::OpenedProjectError(Box::new(CallError::from(e)))
        })?)
    }

    pub async fn close(
        &self,
        session: SessionHandle,
        notifier_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        let res = self
            .0
            .ask(Close {
                session,
                notifier_id,
            })
            .try_send()
            .await;

        Ok(res.map_err(|e| {
            ProjectManagerActorError::OpenedProjectError(Box::new(CallError::from(e)))
        })?)
    }

    pub async fn get_model(&self) -> Result<Arc<model::Project>, ProjectManagerActorError> {
        let res = self.0.ask(Get).try_send().await;

        Ok(res.map_err(|e| ProjectManagerActorError::ActorCommunicationError(CallError::from(e)))?)
    }

    pub async fn call(
        &self,
        session: SessionHandle,
        notifier_id: String,
        call_data: project_manager::ProjectCall,
    ) -> Result<project_manager::ProjectCallResult, ProjectManagerActorError> {
        let res = self
            .0
            .ask(Call {
                call_data,
                session,
                notifier_id,
            })
            .try_send()
            .await;

        Ok(res.map_err(|e| {
            ProjectManagerActorError::OpenedProjectError(Box::new(CallError::from(e)))
        })?)
    }

    pub async fn session_event(&self, event: SessionEvent) -> Vec<String> {
        let res = self.0.ask(event).try_send().await;

        res.unwrap_or_else(|error| {
            tracing::error!(%error, "could not send session event to project actor");
            Vec::new()
        })
    }
}

#[derive(Debug)]
struct CoreProjectArgs {
    name: String,
    data: Arc<project_manager::CoreProject>,
    model: model::Project,
    fs_collection: FsCollectionHandle<project_manager::CoreProject>,
}

/// Represents an opened core project
#[derive(Debug)]
struct CoreProject {
    name: String,
    notifiers: NotifierManager<project_manager::UpdateProjectNotification>,
    /// The stored project, and the domain model of it. Two views of one
    /// project, replaced together and never apart, as in the UI project.
    data: Arc<project_manager::CoreProject>,
    model: model::Project,
    fs_collection: FsCollectionHandle<project_manager::CoreProject>,
}

impl Actor for CoreProject {
    type Args = CoreProjectArgs;
    type Error = Infallible;

    async fn on_start(args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        args.fs_collection.on_update().subscribe(actor_ref);

        Ok(Self {
            name: args.name,
            notifiers: NotifierManager::new("project-manager/opened-project"),
            data: args.data,
            model: args.model,
            fs_collection: args.fs_collection,
        })
    }
}

#[derive(Debug)]
struct Open {
    session: SessionHandle,
}

impl message::Message<Open> for CoreProject {
    type Reply = Result<(String, InitialEmitter), ProjectManagerActorError>;

    async fn handle(&mut self, msg: Open, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let mut changes = Vec::new();
        self.model.initial_changes(&mut changes);
        let notifications = model::wire::notifications(&changes);

        let notifier = self.notifiers.create_notifier(msg.session);

        let notifier_id = notifier.notifier_id().to_owned();
        let notification_emitter = InitialEmitter::new(notifier.clone(), notifications);

        Ok((notifier_id, notification_emitter))
    }
}

#[derive(Debug)]
struct Close {
    session: SessionHandle,
    notifier_id: String,
}

impl message::Message<Close> for CoreProject {
    type Reply = Result<(), ProjectManagerActorError>;

    async fn handle(&mut self, msg: Close, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let removed = self
            .notifiers
            .remove_notifier(&msg.session, &msg.notifier_id);
        if !removed {
            return Err(ProjectManagerActorError::NotifierNotFound(msg.notifier_id));
        }

        Ok(())
    }
}

#[derive(Debug)]
struct Get;

impl message::Message<Get> for CoreProject {
    type Reply = Arc<model::Project>;

    async fn handle(&mut self, _msg: Get, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        Arc::new(self.model.clone())
    }
}

#[derive(Debug)]
struct Call {
    session: SessionHandle,
    notifier_id: String,
    call_data: project_manager::ProjectCall,
}

impl message::Message<Call> for CoreProject {
    type Reply = Result<project_manager::ProjectCallResult, ProjectManagerActorError>;

    async fn handle(&mut self, msg: Call, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let Some(notifier) = self.notifiers.get_notifier(&msg.session, &msg.notifier_id) else {
            return Err(ProjectManagerActorError::NotifierNotFound(
                msg.notifier_id.clone(),
            ));
        };

        if notifier.session_id() != msg.session.id() {
            return Err(ProjectManagerActorError::NotifierNotFound(
                msg.notifier_id.clone(),
            ));
        }

        let call_data =
            serde_json::from_value::<project_manager::CoreProjectCall>(msg.call_data.0)?;

        fn map_call_result<T: Serialize>(
            result: Result<T, ProjectManagerActorError>,
        ) -> Result<project_manager::ProjectCallResult, ProjectManagerActorError> {
            match result {
                Ok(value) => Ok(project_manager::ProjectCallResult(serde_json::to_value(
                    value,
                )?)),
                Err(err) => Err(err),
            }
        }

        use project_manager::CoreProjectCall as Call;

        match call_data {
            Call::UpdateToolbox(call_data) => map_call_result(self.update_toolbox(call_data).await),
            Call::SetTemplate(call_data) => map_call_result(self.set_template(call_data).await),
            Call::ClearTemplate(call_data) => map_call_result(self.clear_template(call_data).await),
            Call::RenameTemplate(call_data) => {
                map_call_result(self.rename_template(call_data).await)
            }
            Call::SetTemplateExport(call_data) => {
                map_call_result(self.set_template_export(call_data).await)
            }
            Call::SetComponent(call_data) => map_call_result(self.set_component(call_data).await),
            Call::MoveComponents(call_data) => {
                map_call_result(self.move_components(call_data).await)
            }
            Call::ConfigureComponent(call_data) => {
                map_call_result(self.configure_component(call_data).await)
            }
            Call::RenameComponent(call_data) => {
                map_call_result(self.rename_component(call_data).await)
            }
            Call::ClearComponents(call_data) => {
                map_call_result(self.clear_components(call_data).await)
            }
            Call::CopyComponentsToTemplate(call_data) => {
                map_call_result(self.copy_components_to_template(call_data).await)
            }
            Call::SetBinding(call_data) => map_call_result(self.set_binding(call_data).await),
            Call::ClearBinding(call_data) => map_call_result(self.clear_binding(call_data).await),
            Call::ClearTemplateExport(call_data) => {
                map_call_result(self.clear_template_export(call_data).await)
            }
            Call::PrepareImportFromOnline(call_data) => {
                map_call_result(self.prepare_import_from_online(call_data).await)
            }
            Call::PrepareImportFromProject(call_data) => {
                map_call_result(self.prepare_import_from_project(call_data).await)
            }
            Call::ApplyBulkUpdates(call_data) => {
                map_call_result(self.apply_bulk_updates(call_data).await)
            }
            Call::Validate => map_call_result(self.validate().await),
            Call::PrepareDeploy => map_call_result(self.prepare_deploy().await),
            Call::ApplyDeploy(call_data) => map_call_result(self.apply_deploy(call_data).await),
        }
    }
}

impl message::Message<Update<project_manager::CoreProject>> for CoreProject {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: Update<project_manager::CoreProject>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if msg.id != self.name {
            return;
        }

        match msg.kind {
            fs_collection::Kind::Created { .. } => {
                tracing::warn!(name = self.name, "unexpected creation of project");
            }
            fs_collection::Kind::Deleted => {
                tracing::warn!(name = self.name, "unexpected deletion of project");
            }

            fs_collection::Kind::Updated { value } => {
                // Was it our update?
                if Arc::ptr_eq(&self.data, &value) {
                    return;
                }

                self.reload(value);
            }

            fs_collection::Kind::Renamed { new_id } => {
                let mut old_name = new_id;
                mem::swap(&mut self.name, &mut old_name);

                tracing::debug!(old_name, new_name = self.name, "project renamed");

                self.notifiers
                    .notify_all(&project_manager::UpdateProjectNotification::SetName(
                        project_manager::SetNameProjectNotification {
                            name: self.name.clone(),
                        },
                    ));
            }
        }
    }
}

impl CoreProject {
    /// Take a project written underneath us.
    ///
    /// A project the model refuses is not taken: the pair in memory stays the
    /// one that was last known to hold together, and nothing is announced.
    fn reload(&mut self, data: Arc<project_manager::CoreProject>) {
        let model = match model::Project::try_from(data.as_ref()) {
            Ok(model) => model,
            Err(error) => {
                tracing::error!(
                    %error,
                    name = self.name,
                    "core project written outside the studio does not fit the model, keeping the one in memory"
                );
                return;
            }
        };

        self.data = data;
        self.model = model;

        let mut changes = vec![model::Change::Reset];
        self.model.initial_changes(&mut changes);

        self.notify_all(&model::wire::notifications(&changes));
    }

    fn notify_all(&mut self, notifications: &[project_manager::UpdateProjectNotification]) {
        for notification in notifications {
            self.notifiers.notify_all(notification);
        }
    }

    /// Run one operation against the domain model, store what it produced and
    /// announce what it changed.
    ///
    /// Same shape as the UI project's bridge: the operation can only fail with
    /// a `CoreModelError`, and anything it records is turned into notifications in
    /// one place.
    async fn apply_model_update<T>(
        &mut self,
        operation: impl FnOnce(
            &model::Project,
            &mut Vec<model::Change>,
        ) -> Result<Option<(model::Project, T)>, model::CoreModelError>,
    ) -> Result<Option<T>, ProjectManagerActorError> {
        let mut changes = Vec::new();
        let Some((model, result)) = operation(&self.model, &mut changes)? else {
            return Ok(None);
        };

        let data = Arc::new(project_manager::CoreProject::from(&model));

        self.fs_collection
            .update(self.name.clone(), data.clone())
            .await?;

        self.data = data;
        self.model = model;

        self.notify_all(&model::wire::notifications(&changes));

        Ok(Some(result))
    }

    /// The common case: an operation that returns nothing.
    async fn apply(
        &mut self,
        operation: impl FnOnce(
            &model::Project,
            &mut Vec<model::Change>,
        ) -> Result<Option<model::Project>, model::CoreModelError>,
    ) -> Result<(), ProjectManagerActorError> {
        self.apply_model_update(|project, changes| {
            Ok(operation(project, changes)?.map(|project| (project, ())))
        })
        .await?;

        Ok(())
    }
}

impl message::Message<SessionEvent> for CoreProject {
    type Reply = Vec<String>;

    async fn handle(
        &mut self,
        msg: SessionEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.notifiers.session_event(&msg)
    }
}

// ---------------------------------------------------------------------------
// Operations
//
// Each one is the same three steps: turn the untyped call payload into typed
// values, run one model operation, let the bridge store and notify.
// ---------------------------------------------------------------------------

impl CoreProject {
    async fn update_toolbox(
        &mut self,
        call_data: project_manager::UpdateToolboxCoreProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let project_manager::UpdateToolboxCoreProjectCall {
            item_type,
            item_id,
            action,
        } = call_data;

        match (item_type.as_str(), action.as_str()) {
            ("instance", "show") | ("instance", "hide") => {
                let instance = model::InstanceId::new(&item_id)?;
                let display = toolbox_display(&action)?;

                self.apply(|project, changes| {
                    project.set_instance_display(changes, &instance, display)
                })
                .await
            }

            ("instance", "delete") => {
                let instance = model::InstanceId::new(&item_id)?;

                self.apply(|project, changes| {
                    project.clear_unused_instance_plugins(changes, &instance)
                })
                .await
            }

            ("plugin", "show") | ("plugin", "hide") => {
                let plugin = model::PluginId::new(&item_id)?;
                let display = toolbox_display(&action)?;

                self.apply(|project, changes| project.set_plugin_display(changes, &plugin, display))
                    .await
            }

            ("plugin", "delete") => {
                let plugin = model::PluginId::new(&item_id)?;

                self.apply(|project, changes| project.clear_plugin(changes, &plugin))
                    .await
            }

            ("instance", _) | ("plugin", _) => Err(model::CoreModelError::UnknownValue {
                kind: "toolbox action",
                value: action,
            }
            .into()),

            _ => Err(model::CoreModelError::UnknownValue {
                kind: "toolbox item type",
                value: item_type,
            }
            .into()),
        }
    }

    async fn set_template(
        &mut self,
        call_data: project_manager::SetTemplateCoreProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::TemplateId::new(&call_data.template_id)?;

        self.apply(|project, changes| project.set_template(changes, id).map(Some))
            .await
    }

    async fn clear_template(
        &mut self,
        call_data: project_manager::ClearTemplateCoreProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::TemplateId::new(&call_data.template_id)?;

        self.apply(|project, changes| project.clear_template(changes, &id))
            .await
    }

    async fn rename_template(
        &mut self,
        call_data: project_manager::RenameTemplateCoreProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::TemplateId::new(&call_data.template_id)?;
        let new_id = model::TemplateId::new(&call_data.new_id)?;

        self.apply(|project, changes| project.rename_template(changes, &id, new_id))
            .await
    }

    async fn set_template_export(
        &mut self,
        call_data: project_manager::SetTemplateExportCoreProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let template_id = model::TemplateId::new(&call_data.template_id)?;
        let kind = model::ExportKind::parse(&call_data.export_type)?;
        let export_id = model::ExportId::new(&call_data.export_id)?;
        let component_id = model::ComponentId::new(&call_data.component_id)?;

        self.apply(|project, changes| {
            project
                .set_template_export(
                    changes,
                    &template_id,
                    kind,
                    export_id,
                    &component_id,
                    &call_data.property_name,
                )
                .map(Some)
        })
        .await
    }

    async fn set_component(
        &mut self,
        call_data: project_manager::SetComponentCoreProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(&call_data.template_id)?;
        let id = model::ComponentId::new(&call_data.component_id)?;
        let definition = model::ComponentDefinition::try_from(&call_data.definition)?;
        let position = model::Position {
            x: call_data.x,
            y: call_data.y,
        };

        self.apply(|project, changes| {
            project
                .set_component(changes, &view, id, definition, position)
                .map(Some)
        })
        .await
    }

    async fn move_components(
        &mut self,
        call_data: project_manager::MoveComponentsCoreProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(&call_data.template_id)?;
        let ids = model::wire::component_ids(&call_data.components_ids)?;
        let delta = model::Position {
            x: call_data.delta.x,
            y: call_data.delta.y,
        };

        self.apply(|project, changes| {
            project
                .move_components(changes, &view, &ids, delta)
                .map(Some)
        })
        .await
    }

    async fn configure_component(
        &mut self,
        call_data: project_manager::ConfigureComponentCoreProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(&call_data.template_id)?;
        let id = model::ComponentId::new(&call_data.component_id)?;

        self.apply(|project, changes| {
            project
                .configure_component(
                    changes,
                    &view,
                    id,
                    &call_data.config_id,
                    call_data.config_value,
                )
                .map(Some)
        })
        .await
    }

    async fn rename_component(
        &mut self,
        call_data: project_manager::RenameComponentCoreProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(&call_data.template_id)?;
        let id = model::ComponentId::new(&call_data.component_id)?;
        let new_id = model::ComponentId::new(&call_data.new_id)?;

        self.apply(|project, changes| {
            project
                .rename_component(changes, &view, &id, new_id)
                .map(Some)
        })
        .await
    }

    async fn clear_components(
        &mut self,
        call_data: project_manager::ClearComponentsCoreProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(&call_data.template_id)?;
        let ids = model::wire::component_ids(&call_data.components_ids)?;

        self.apply(|project, changes| project.clear_components(changes, &view, &ids).map(Some))
            .await
    }

    async fn copy_components_to_template(
        &mut self,
        call_data: project_manager::CopyComponentsToTemplateCoreProjectCall,
    ) -> Result<project_manager::CopyComponentsCoreProjectCallResult, ProjectManagerActorError>
    {
        let source = model::wire::view_key(&call_data.template_id)?;
        let target = model::wire::view_key(&call_data.target_template_id)?;
        let ids = model::wire::component_ids(&call_data.components_ids)?;

        let stats = self
            .apply_model_update(|project, changes| {
                let (project, components, bindings) =
                    project.copy_components(changes, &source, &ids, &target)?;

                Ok(Some((
                    project,
                    project_manager::CopyComponentsStats {
                        components: components as i32,
                        bindings: bindings as i32,
                    },
                )))
            })
            .await?
            .expect("copy always reports its result");

        Ok(project_manager::CopyComponentsCoreProjectCallResult { stats })
    }

    async fn set_binding(
        &mut self,
        call_data: project_manager::SetBindingCoreProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(&call_data.template_id)?;
        let binding = model::Binding::try_from(&call_data.binding)?;

        self.apply(|project, changes| project.set_binding(changes, &view, binding).map(Some))
            .await
    }

    async fn clear_binding(
        &mut self,
        call_data: project_manager::ClearBindingCoreProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(&call_data.template_id)?;
        let id = model::wire::binding_id(&call_data.binding_id)?;

        self.apply(|project, changes| project.clear_binding(changes, &view, &id))
            .await
    }

    /// Everything wrong with the project, as the designer's check reports it.
    ///
    /// The deploy path runs the same pass with different options: it has
    /// somewhere to put the bindings to worry about, and it cannot proceed
    /// over a difference this one only reports.
    async fn validate(
        &mut self,
    ) -> Result<project_manager::ValidateCoreProjectCallResult, ProjectManagerActorError> {
        let resolved = resolver::resolve(&self.model)?;
        let online = online_snapshot(true).await?;

        let validation = validation::validate(
            &resolved,
            &online,
            validation::Options {
                online_severity: Severity::Error,
                check_bindings_instance: true,
            },
        )?;

        Ok(project_manager::ValidateCoreProjectCallResult { validation })
    }

    /// What taking in what is running would change.
    async fn prepare_import_from_online(
        &mut self,
        call_data: project_manager::PrepareImportFromOnlineCoreProjectCall,
    ) -> Result<project_manager::PrepareBulkUpdatesCoreProjectCallResult, ProjectManagerActorError>
    {
        let source = online_source().await?;
        let imports = import::from_online(&source, &call_data.config)?;

        self.prepare_bulk_updates(imports)
    }

    /// What taking in another project would change.
    async fn prepare_import_from_project(
        &mut self,
        call_data: project_manager::PrepareImportFromProjectCoreProjectCall,
    ) -> Result<project_manager::PrepareBulkUpdatesCoreProjectCallResult, ProjectManagerActorError>
    {
        let manager = ProjectManagerHandler::new()?;
        let source = manager
            .get_project_model(
                project_manager::ProjectType::Core,
                call_data.config.project_id.clone(),
            )
            .await?;

        let ProjectModel::Core(source) = source else {
            unreachable!("asked for a core project, got a ui one")
        };

        // Flattened, because an import is about what runs.
        let source = resolver::resolve(&source)?;
        let imports = import::from_project(&source, &call_data.config)?;

        self.prepare_bulk_updates(imports)
    }

    /// Removing a template export is prepared like an import: it drags enough
    /// along that the designer is shown the whole of it first.
    async fn clear_template_export(
        &mut self,
        call_data: project_manager::ClearTemplateExportCoreProjectCall,
    ) -> Result<project_manager::PrepareBulkUpdatesCoreProjectCallResult, ProjectManagerActorError>
    {
        let change = project_manager::core_import_data::ObjectChange::Template(
            project_manager::core_import_data::TemplateChange {
                // There is only ever this one change, so the key is fixed.
                key: String::from("update-template"),
                id: call_data.template_id,
                change_type: project_manager::core_import_data::ChangeType::Update,
                dependencies: Vec::new(),
                impacts: Vec::new(),
                export_type: call_data.export_type,
                export_id: call_data.export_id,
            },
        );

        self.bulk_updates(import::ImportData::default(), vec![change])
    }

    /// Work out every write the changes call for, and hand both back.
    fn prepare_bulk_updates(
        &mut self,
        imports: import::ImportData,
    ) -> Result<project_manager::PrepareBulkUpdatesCoreProjectCallResult, ProjectManagerActorError>
    {
        let changes = import::prepare_changes(&imports, &self.model)?;
        self.bulk_updates(imports, changes)
    }

    fn bulk_updates(
        &mut self,
        imports: import::ImportData,
        mut changes: Vec<project_manager::core_import_data::ObjectChange>,
    ) -> Result<project_manager::PrepareBulkUpdatesCoreProjectCallResult, ProjectManagerActorError>
    {
        let updates = import::build_updates(&imports, &self.model, &mut changes)?;

        Ok(project_manager::PrepareBulkUpdatesCoreProjectCallResult {
            changes,
            server_data: serde_json::to_value(&updates)?,
        })
    }

    /// Write the changes the designer picked.
    async fn apply_bulk_updates(
        &mut self,
        call_data: project_manager::ApplyBulkUpdatesCoreProject,
    ) -> Result<project_manager::ApplyBulkUpdatesCoreProjectCallResult, ProjectManagerActorError>
    {
        let updates: Vec<import::Update> = serde_json::from_value(call_data.server_data)?;
        let selection: BTreeSet<String> = call_data.selection.into_iter().collect();

        let stats = self
            .apply_model_update(|project, changes| {
                let (project, stats) =
                    import::apply_updates(project, &updates, &selection, changes)?;

                Ok(Some((project, stats)))
            })
            .await?
            .unwrap_or(project_manager::BulkUpdatesStats {
                plugins: 0,
                components: 0,
                templates: 0,
                bindings: 0,
            });

        Ok(project_manager::ApplyBulkUpdatesCoreProjectCallResult { stats })
    }

    /// What deploying onto the running instances would change.
    ///
    /// Nothing is touched here. The task list goes back to the designer inside
    /// `server_data` and comes back untouched on the apply call, so what the
    /// user agreed to is what runs.
    async fn prepare_deploy(
        &mut self,
    ) -> Result<project_manager::PrepareDeployCoreProjectCallResult, ProjectManagerActorError> {
        let resolved = resolver::resolve(&self.model)?;
        let online = online_snapshot(true).await?;

        let validation = validation::validate(
            &resolved,
            &online,
            validation::Options {
                online_severity: Severity::Error,
                check_bindings_instance: true,
            },
        )?;

        if Item::has_error(&validation) {
            return Ok(project_manager::PrepareDeployCoreProjectCallResult {
                validation,
                changes: DeployChanges {
                    components: Vec::new(),
                    bindings: Vec::new(),
                },
                server_data: serde_json::Value::Null,
            });
        }

        let store = online_store(&resolved).await?;
        let plan = deploy::prepare_to_online(&resolved, &store)?;

        Ok(project_manager::PrepareDeployCoreProjectCallResult {
            validation,
            changes: plan.changes,
            server_data: serde_json::to_value(&plan.tasks)?,
        })
    }

    /// Carry out the plan the designer agreed to.
    ///
    /// Each instance that was touched then saves its store, so the deploy
    /// survives a restart.
    async fn apply_deploy(
        &mut self,
        call_data: project_manager::ApplyDeployCoreProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let tasks: Vec<deploy::Task> = serde_json::from_value(call_data.server_data)?;

        let resolved = resolver::resolve(&self.model)?;
        let instances = OnlineInstancesHandle::new()?;
        let mut touched = BTreeSet::new();

        for task in tasks {
            touched.insert(task.instance_name.clone());

            match (&task.object, task.change_type) {
                (deploy::TaskObject::Component { id }, deploy::TaskChange::Add) => {
                    let config =
                        deploy::component_config(&resolved, &model::ComponentId::new(id)?)?;
                    instances
                        .core_add_component(task.instance_name, config)
                        .await?;
                }

                (deploy::TaskObject::Component { id }, deploy::TaskChange::Delete) => {
                    instances
                        .core_remove_component(task.instance_name, id.clone())
                        .await?;
                }

                (deploy::TaskObject::Binding { id }, deploy::TaskChange::Add) => {
                    let config = deploy::binding_config(&model::BindingId::new(id)?)?;
                    instances
                        .core_add_binding(task.instance_name, config)
                        .await?;
                }

                (deploy::TaskObject::Binding { id }, deploy::TaskChange::Delete) => {
                    let config = deploy::binding_config(&model::BindingId::new(id)?)?;
                    instances
                        .core_remove_binding(task.instance_name, config)
                        .await?;
                }
            }
        }

        for instance in touched {
            instances.core_store_save(instance.clone()).await?;
            tracing::debug!(instance, project = self.name, "core store saved");
        }

        Ok(())
    }
}

/// Every plugin and component the registry knows about.
async fn online_source() -> Result<import::OnlineSource, ProjectManagerActorError> {
    let registry = RegistryHandle::new()?;

    let mut plugins = Vec::new();

    for plugin in registry.get_plugins().await? {
        let Some(instance) = plugin.instance else {
            continue;
        };

        plugins.push((model::InstanceId::new(instance)?, plugin.metadata));
    }

    let mut components = Vec::new();

    for component in registry.get_components().await? {
        let Some(instance) = component.instance else {
            continue;
        };

        components.push(import::OnlineComponent {
            id: model::ComponentId::new(component.component_id)?,
            instance: model::InstanceId::new(instance)?,
            plugin: component.plugin,
        });
    }

    Ok(import::OnlineSource {
        plugins,
        components,
    })
}

/// What the instances currently hold, for the instances this project deploys
/// to and for the one carrying the bindings.
async fn online_store(
    project: &resolver::ResolvedProject,
) -> Result<deploy::OnlineStore, ProjectManagerActorError> {
    let instances = OnlineInstancesHandle::new()?;

    let (bindings_instance, bindings) = if project.bindings().is_empty() {
        (None, Vec::new())
    } else {
        // Validation has already established there is exactly one.
        let carriers = instances
            .instances_with_capability(validation::BINDINGS_API.to_owned())
            .await?;

        match carriers.into_iter().next() {
            Some(instance) => {
                let bindings = instances.core_list_bindings(instance.clone()).await?;
                (Some(instance), bindings)
            }
            None => return Err(model::CoreModelError::BindingsInstanceNeeded.into()),
        }
    };

    let mut components = BTreeMap::new();

    for instance in project.instances()?.keys() {
        components.insert(
            instance.to_string(),
            instances.core_list_components(instance.to_string()).await?,
        );
    }

    Ok(deploy::OnlineStore {
        bindings_instance,
        bindings,
        components,
    })
}

/// What the project is checked against, read once so the checks themselves
/// stay a function of two values.
async fn online_snapshot(
    check_bindings_instance: bool,
) -> Result<validation::Online, ProjectManagerActorError> {
    let registry = RegistryHandle::new()?;

    // A plugin or a component with no instance is local to this studio, so it
    // is not something a project deploys against.
    let mut plugins = Vec::new();

    for plugin in registry.get_plugins().await? {
        let Some(instance) = plugin.instance else {
            continue;
        };

        let import =
            import::plugin_from_metadata(&model::InstanceId::new(instance)?, &plugin.metadata)?;
        plugins.push((import.id, Arc::new(import.plugin)));
    }

    let mut components = Vec::new();

    for component in registry.get_components().await? {
        let Some(instance) = component.instance else {
            continue;
        };

        let instance = model::InstanceId::new(instance)?;
        let import = import::plugin_from_metadata(&instance, &component.plugin)?;

        components.push((
            model::ComponentId::new(component.component_id)?,
            validation::OnlineComponent {
                instance,
                plugin: Arc::new(import.plugin),
            },
        ));
    }

    let bindings_instances = if check_bindings_instance {
        OnlineInstancesHandle::new()?
            .instances_with_capability(validation::BINDINGS_API.to_owned())
            .await?
    } else {
        Vec::new()
    };

    Ok(validation::Online::new(
        plugins,
        components,
        bindings_instances,
    ))
}

fn toolbox_display(action: &str) -> Result<model::ToolboxDisplay, model::CoreModelError> {
    match action {
        "show" => Ok(model::ToolboxDisplay::Show),
        "hide" => Ok(model::ToolboxDisplay::Hide),
        _ => Err(model::CoreModelError::UnknownValue {
            kind: "toolbox action",
            value: action.to_owned(),
        }),
    }
}
