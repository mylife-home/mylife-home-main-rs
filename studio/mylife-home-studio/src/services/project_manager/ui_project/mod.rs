use std::{mem, sync::Arc, unreachable};

use common::utils::actors::{CallError, SpawnExt};
use serde::Serialize;
use std::collections::HashMap;
use studio_web_api::project_manager;

use crate::{
    services::{
        online::OnlineInstancesHandle,
        project_manager::{
            InitialEmitter, PendingCall, ProjectManagerActorError, ProjectManagerHandler,
            ProjectModel,
        },
    },
    utils::fs_collection::{self, FsCollectionHandle, Update},
    web::{NotifierManager, SessionEvent, SessionHandle},
};

use kameo::{error::Infallible, message, prelude::*};

mod component;
mod deploy;
pub mod model;

#[cfg(test)]
mod tests;

pub fn new_ui_project_data() -> Arc<project_manager::UiProject> {
    Arc::new(project_manager::UiProject {
        resources: Arc::new(HashMap::new()),
        styles: Arc::new(HashMap::new()),
        windows: Arc::new(HashMap::new()),
        templates: Arc::new(HashMap::new()),
        default_window: Arc::new(project_manager::UiDefaultWindowData(HashMap::from_iter([
            ("desktop".to_string(), None),
            ("mobile".to_string(), None),
        ]))),
        components: Arc::new(HashMap::new()),
        plugins: Arc::new(HashMap::new()),
    })
}

#[derive(Debug, Clone)]
pub struct UiProjectHandle(ActorRef<UiProject>);

impl UiProjectHandle {
    pub async fn create(
        name: String,
        fs_collection: FsCollectionHandle<project_manager::UiProject>,
    ) -> Result<Self, ProjectManagerActorError> {
        let data = fs_collection.get(name.clone()).await?;

        // A stored project the model refuses cannot be served at all, so it
        // fails here rather than on whichever call happens to touch it first.
        let model = model::Project::try_from(data.as_ref())?;

        let actor = UiProject::spawn_unbounded(UiProjectArgs {
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

    /// Puts the call on the actor's mailbox and hands back its reply.
    ///
    /// The mailbox position is taken here, so calls keep the order they
    /// arrived in; only the waiting happens elsewhere.
    pub fn enqueue_call(
        &self,
        session: SessionHandle,
        notifier_id: String,
        call_data: project_manager::ProjectCall,
    ) -> Result<PendingCall, ProjectManagerActorError> {
        let pending = self
            .0
            .ask(Call {
                call_data,
                session,
                notifier_id,
            })
            .try_enqueue()
            .map_err(|error| {
                ProjectManagerActorError::OpenedProjectError(Box::new(CallError::from(
                    error.map_err(|never: Infallible| match never {}),
                )))
            })?;

        Ok(Box::pin(async move {
            pending.await.map_err(|error| {
                ProjectManagerActorError::OpenedProjectError(Box::new(CallError::from(error)))
            })
        }))
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
struct UiProjectArgs {
    name: String,
    data: Arc<project_manager::UiProject>,
    model: model::Project,
    fs_collection: FsCollectionHandle<project_manager::UiProject>,
}

/// Represents an opened ui project
#[derive(Debug)]
struct UiProject {
    name: String,
    notifiers: NotifierManager<project_manager::UpdateProjectNotification>,
    /// The stored project, and the domain model of it.
    ///
    /// Two views of one project, replaced together and never apart, so nothing
    /// below has to ask whether they agree. `data` is what `fs_collection`
    /// writes and hands back; `model` is what the operations run on.
    data: Arc<project_manager::UiProject>,
    model: model::Project,
    fs_collection: FsCollectionHandle<project_manager::UiProject>,
}

impl Actor for UiProject {
    type Args = UiProjectArgs;
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

impl message::Message<Open> for UiProject {
    type Reply = Result<(String, InitialEmitter), ProjectManagerActorError>;

    async fn handle(&mut self, msg: Open, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        // Built before the notifier, which borrows `self.notifiers`.
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

impl message::Message<Close> for UiProject {
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

impl message::Message<Get> for UiProject {
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

impl message::Message<Call> for UiProject {
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

        let call_data = serde_json::from_value::<project_manager::UiProjectCall>(msg.call_data.0)?;

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

        match call_data {
            project_manager::UiProjectCall::Validate => map_call_result(self.validate().await),
            project_manager::UiProjectCall::RefreshComponentsFromOnline => {
                map_call_result(self.refresh_components_from_online().await)
            }
            project_manager::UiProjectCall::RefreshComponentsFromProject(call_data) => {
                map_call_result(self.refresh_components_from_project(call_data).await)
            }
            project_manager::UiProjectCall::ApplyRefreshComponents(call_data) => {
                map_call_result(self.apply_refresh_components(call_data).await)
            }
            project_manager::UiProjectCall::Deploy => map_call_result(self.deploy().await),
            project_manager::UiProjectCall::SetDefaultWindow(call_data) => {
                map_call_result(self.set_default_window(call_data).await)
            }

            project_manager::UiProjectCall::SetResource(call_data) => {
                map_call_result(self.set_resource(call_data).await)
            }
            project_manager::UiProjectCall::ClearResource(call_data) => {
                map_call_result(self.clear_resource(call_data).await)
            }
            project_manager::UiProjectCall::RenameResource(call_data) => {
                map_call_result(self.rename_resource(call_data).await)
            }

            project_manager::UiProjectCall::SetStyle(call_data) => {
                map_call_result(self.set_style(call_data).await)
            }
            project_manager::UiProjectCall::ClearStyle(call_data) => {
                map_call_result(self.clear_style(call_data).await)
            }
            project_manager::UiProjectCall::RenameStyle(call_data) => {
                map_call_result(self.rename_style(call_data).await)
            }

            project_manager::UiProjectCall::NewWindow(call_data) => {
                map_call_result(self.new_window(call_data).await)
            }
            project_manager::UiProjectCall::ClearWindow(call_data) => {
                map_call_result(self.clear_window(call_data).await)
            }
            project_manager::UiProjectCall::RenameWindow(call_data) => {
                map_call_result(self.rename_window(call_data).await)
            }
            project_manager::UiProjectCall::CloneWindow(call_data) => {
                map_call_result(self.clone_window(call_data).await)
            }
            project_manager::UiProjectCall::SetWindowProperties(call_data) => {
                map_call_result(self.set_window_properties(call_data).await)
            }

            project_manager::UiProjectCall::NewTemplate(call_data) => {
                map_call_result(self.new_template(call_data).await)
            }
            project_manager::UiProjectCall::ClearTemplate(call_data) => {
                map_call_result(self.clear_template(call_data).await)
            }
            project_manager::UiProjectCall::RenameTemplate(call_data) => {
                map_call_result(self.rename_template(call_data).await)
            }
            project_manager::UiProjectCall::CloneTemplate(call_data) => {
                map_call_result(self.clone_template(call_data).await)
            }
            project_manager::UiProjectCall::SetTemplateProperties(call_data) => {
                map_call_result(self.set_template_properties(call_data).await)
            }
            project_manager::UiProjectCall::SetTemplateExport(call_data) => {
                map_call_result(self.set_template_export(call_data).await)
            }
            project_manager::UiProjectCall::ClearTemplateExport(call_data) => {
                map_call_result(self.clear_template_export(call_data).await)
            }
            project_manager::UiProjectCall::SetTemplateBulkPatterns(call_data) => {
                map_call_result(self.set_template_bulk_patterns(call_data).await)
            }

            project_manager::UiProjectCall::NewControl(call_data) => {
                map_call_result(self.new_control(call_data).await)
            }
            project_manager::UiProjectCall::ClearControl(call_data) => {
                map_call_result(self.clear_control(call_data).await)
            }
            project_manager::UiProjectCall::RenameControl(call_data) => {
                map_call_result(self.rename_control(call_data).await)
            }
            project_manager::UiProjectCall::CloneControl(call_data) => {
                map_call_result(self.clone_control(call_data).await)
            }
            project_manager::UiProjectCall::SetControlProperties(call_data) => {
                map_call_result(self.set_control_properties(call_data).await)
            }

            project_manager::UiProjectCall::NewTemplateInstance(call_data) => {
                map_call_result(self.new_template_instance(call_data).await)
            }
            project_manager::UiProjectCall::ClearTemplateInstance(call_data) => {
                map_call_result(self.clear_template_instance(call_data).await)
            }
            project_manager::UiProjectCall::RenameTemplateInstance(call_data) => {
                map_call_result(self.rename_template_instance(call_data).await)
            }
            project_manager::UiProjectCall::CloneTemplateInstance(call_data) => {
                map_call_result(self.clone_template_instance(call_data).await)
            }
            project_manager::UiProjectCall::MoveTemplateInstance(call_data) => {
                map_call_result(self.move_template_instance(call_data).await)
            }
            project_manager::UiProjectCall::SetTemplateInstanceTemplate(call_data) => {
                map_call_result(self.set_template_instance_template(call_data).await)
            }
            project_manager::UiProjectCall::SetTemplateInstanceBindings(call_data) => {
                map_call_result(self.set_template_instance_bindings(call_data).await)
            }
        }
    }
}

impl message::Message<Update<project_manager::UiProject>> for UiProject {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: Update<project_manager::UiProject>,
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

impl message::Message<SessionEvent> for UiProject {
    type Reply = Vec<String>;

    async fn handle(
        &mut self,
        msg: SessionEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.notifiers.session_event(&msg)
    }
}

impl UiProject {
    /// Take a project written underneath us.
    ///
    /// A project the model refuses is not taken: the pair in memory stays the
    /// one that was last known to hold together, and nothing is announced.
    /// Serving a project no call can operate on would be worse.
    fn reload(&mut self, data: Arc<project_manager::UiProject>) {
        let model = match model::Project::try_from(data.as_ref()) {
            Ok(model) => model,
            Err(error) => {
                tracing::error!(
                    %error,
                    name = self.name,
                    "UI project written outside the studio does not fit the model, keeping the one in memory"
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
    /// `Ok(None)` from the operation means nothing moved, so nothing is stored
    /// and nothing is notified. An operation that fails has written nothing:
    /// `self` is still the project it was handed.
    ///
    /// The outbound conversion happens per call and is not worth avoiding:
    /// `fs_collection` serialises the whole project to JSON and rewrites its
    /// file every time, so a `UiProject` has to exist anyway, and building one
    /// from the model is a rounding error next to that write.
    async fn apply_model_update(
        &mut self,
        operation: impl FnOnce(
            &model::Project,
            &mut Vec<model::Change>,
        ) -> Result<Option<model::Project>, model::UiModelError>,
    ) -> Result<(), ProjectManagerActorError> {
        let mut changes = Vec::new();
        let Some(model) = operation(&self.model, &mut changes)? else {
            return Ok(());
        };

        let data = Arc::new(project_manager::UiProject::from(&model));

        self.fs_collection
            .update(self.name.clone(), data.clone())
            .await?;

        self.data = data;
        self.model = model;

        self.notify_all(&model::wire::notifications(&changes));

        Ok(())
    }

    async fn validate(
        &mut self,
    ) -> Result<project_manager::ValidateUiProjectCallResult, ProjectManagerActorError> {
        let errors = deploy::deploy(&self.model).err().unwrap_or_default();

        Ok(project_manager::ValidateUiProjectCallResult { errors })
    }

    async fn refresh_components_from_online(
        &mut self,
    ) -> Result<project_manager::RefreshComponentsUiProjectCallResult, ProjectManagerActorError>
    {
        let component_data = component::load_online_component_data().await?;

        let usage = self.model.collect_component_usage();
        let (breaking_operations, server_data) =
            component::prepare_refresh(&self.model, &usage, component_data)?;

        Ok(project_manager::RefreshComponentsUiProjectCallResult {
            breaking_operations,
            server_data,
        })
    }

    async fn refresh_components_from_project(
        &mut self,
        call_data: project_manager::RefreshComponentsFromProjectUiProjectCall,
    ) -> Result<project_manager::RefreshComponentsUiProjectCallResult, ProjectManagerActorError>
    {
        let manager = ProjectManagerHandler::new()?;
        let project = manager
            .get_project_model(project_manager::ProjectType::Core, call_data.project_id)
            .await?;

        let ProjectModel::Core(project) = project else {
            unreachable!("asked for a core project, got something else")
        };

        let component_data = component::load_core_project_component_data(&project)?;

        let usage = self.model.collect_component_usage();
        let (breaking_operations, server_data) =
            component::prepare_refresh(&self.model, &usage, component_data)?;

        Ok(project_manager::RefreshComponentsUiProjectCallResult {
            breaking_operations,
            server_data,
        })
    }

    async fn apply_refresh_components(
        &mut self,
        call_data: project_manager::ApplyRefreshComponentsUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        // Parsed before the model is touched, so the operation itself can only
        // fail with a `UiModelError`.
        let refresh = component::Refresh::parse(call_data.server_data)?;

        self.apply_model_update(|project, changes| refresh.apply(project, changes).map(Some))
            .await
    }

    async fn deploy(
        &mut self,
    ) -> Result<project_manager::DeployUiProjectCallResult, ProjectManagerActorError> {
        let definition = match deploy::deploy(&self.model) {
            Ok(definition) => definition,
            Err(validation_errors) => {
                return Ok(project_manager::DeployUiProjectCallResult {
                    validation_errors: Some(validation_errors),
                    deploy_error: None,
                });
            }
        };

        let online_instances = OnlineInstancesHandle::new()?;

        let deploy_error = match online_instances.deploy_ui_definition(definition).await {
            Ok(_) => None,
            Err(error) => {
                tracing::error!(%error, project = %self.name, "could not deploy UI project");
                Some(error.to_string())
            }
        };

        Ok(project_manager::DeployUiProjectCallResult {
            validation_errors: None,
            deploy_error,
        })
    }

    async fn set_default_window(
        &mut self,
        call_data: project_manager::SetDefaultWindowUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let default_window = model::DefaultWindow::try_from(&call_data.default_window)?;

        self.apply_model_update(|project, changes| {
            Ok(Some(project.set_default_window(changes, default_window)))
        })
        .await
    }

    // -----------------------------------------------------------------------
    // Resources
    // -----------------------------------------------------------------------

    async fn set_resource(
        &mut self,
        call_data: project_manager::SetResourceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::ResourceId::new(&call_data.id)?;
        let resource = model::Resource::try_from(&call_data.resource)?;

        self.apply_model_update(|project, changes| {
            Ok(Some(project.set_resource(changes, id, resource)))
        })
        .await
    }

    async fn clear_resource(
        &mut self,
        call_data: project_manager::ClearResourceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::ResourceId::new(&call_data.id)?;

        self.apply_model_update(|project, changes| Ok(project.clear_resource(changes, &id)))
            .await
    }

    async fn rename_resource(
        &mut self,
        call_data: project_manager::RenameResourceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::ResourceId::new(&call_data.id)?;
        let new_id = model::ResourceId::new(&call_data.new_id)?;

        self.apply_model_update(|project, changes| project.rename_resource(changes, &id, new_id))
            .await
    }

    // -----------------------------------------------------------------------
    // Styles
    // -----------------------------------------------------------------------

    async fn set_style(
        &mut self,
        call_data: project_manager::SetStyleUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::StyleId::new(&call_data.id)?;
        let style = model::StyleDefinition::try_from(&call_data.style)?;

        self.apply_model_update(|project, changes| Ok(Some(project.set_style(changes, id, style))))
            .await
    }

    async fn clear_style(
        &mut self,
        call_data: project_manager::ClearStyleUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::StyleId::new(&call_data.id)?;

        self.apply_model_update(|project, changes| Ok(project.clear_style(changes, &id)))
            .await
    }

    async fn rename_style(
        &mut self,
        call_data: project_manager::RenameStyleUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::StyleId::new(&call_data.id)?;
        let new_id = model::StyleId::new(&call_data.new_id)?;

        self.apply_model_update(|project, changes| project.rename_style(changes, &id, new_id))
            .await
    }

    // -----------------------------------------------------------------------
    // Windows
    // -----------------------------------------------------------------------

    async fn new_window(
        &mut self,
        call_data: project_manager::NewWindowUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::WindowId::new(&call_data.id)?;

        self.apply_model_update(|project, changes| project.new_window(changes, id).map(Some))
            .await
    }

    async fn clear_window(
        &mut self,
        call_data: project_manager::ClearWindowUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::WindowId::new(&call_data.id)?;

        self.apply_model_update(|project, changes| Ok(project.clear_window(changes, &id)))
            .await
    }

    async fn rename_window(
        &mut self,
        call_data: project_manager::RenameWindowUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::WindowId::new(&call_data.id)?;
        let new_id = model::WindowId::new(&call_data.new_id)?;

        self.apply_model_update(|project, changes| project.rename_window(changes, &id, new_id))
            .await
    }

    async fn clone_window(
        &mut self,
        call_data: project_manager::CloneWindowUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::WindowId::new(&call_data.id)?;
        let new_id = model::WindowId::new(&call_data.new_id)?;

        self.apply_model_update(|project, changes| {
            project.clone_window(changes, &id, new_id).map(Some)
        })
        .await
    }

    async fn set_window_properties(
        &mut self,
        call_data: project_manager::SetWindowPropertiesUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::WindowId::new(&call_data.id)?;
        let properties = model::WindowProperties::try_from(call_data.properties)?;

        self.apply_model_update(|project, changes| {
            project.set_window_properties(changes, id, properties)
        })
        .await
    }

    // -----------------------------------------------------------------------
    // Templates
    // -----------------------------------------------------------------------

    async fn new_template(
        &mut self,
        call_data: project_manager::NewTemplateUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::TemplateId::new(&call_data.id)?;

        self.apply_model_update(|project, changes| project.new_template(changes, id).map(Some))
            .await
    }

    async fn clear_template(
        &mut self,
        call_data: project_manager::ClearTemplateUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::TemplateId::new(&call_data.id)?;

        self.apply_model_update(|project, changes| Ok(project.clear_template(changes, &id)))
            .await
    }

    async fn rename_template(
        &mut self,
        call_data: project_manager::RenameTemplateUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::TemplateId::new(&call_data.id)?;
        let new_id = model::TemplateId::new(&call_data.new_id)?;

        self.apply_model_update(|project, changes| project.rename_template(changes, &id, new_id))
            .await
    }

    async fn clone_template(
        &mut self,
        call_data: project_manager::CloneTemplateUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::TemplateId::new(&call_data.id)?;
        let new_id = model::TemplateId::new(&call_data.new_id)?;

        self.apply_model_update(|project, changes| {
            project.clone_template(changes, &id, new_id).map(Some)
        })
        .await
    }

    async fn set_template_properties(
        &mut self,
        call_data: project_manager::SetTemplatePropertiesUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::TemplateId::new(&call_data.id)?;
        let properties = model::TemplateProperties::try_from(call_data.properties)?;

        self.apply_model_update(|project, changes| {
            project.set_template_properties(changes, id, properties)
        })
        .await
    }

    // -----------------------------------------------------------------------
    // Template exports
    // -----------------------------------------------------------------------

    async fn set_template_export(
        &mut self,
        call_data: project_manager::SetTemplateExportUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::TemplateId::new(&call_data.id)?;
        let export_id = model::ExportId::new(&call_data.export_id)?;

        self.apply_model_update(|project, changes| {
            project
                .set_template_export(
                    changes,
                    id,
                    export_id,
                    call_data.member_type,
                    call_data.value_type,
                )
                .map(Some)
        })
        .await
    }

    async fn clear_template_export(
        &mut self,
        call_data: project_manager::ClearTemplateExportUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::TemplateId::new(&call_data.id)?;
        let export_id = model::ExportId::new(&call_data.export_id)?;

        self.apply_model_update(|project, changes| {
            project.clear_template_export(changes, id, &export_id)
        })
        .await
    }

    async fn set_template_bulk_patterns(
        &mut self,
        call_data: project_manager::SetTemplateBulkPatternsUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let id = model::TemplateId::new(&call_data.id)?;
        let patterns = model::wire::bulk_patterns(call_data.patterns)?;

        self.apply_model_update(|project, changes| {
            project.set_template_bulk_patterns(changes, id, &patterns)
        })
        .await
    }

    // -----------------------------------------------------------------------
    // Controls
    // -----------------------------------------------------------------------

    async fn new_control(
        &mut self,
        call_data: project_manager::NewControlUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(call_data.view_type, &call_data.view_id)?;
        let id = model::ControlId::new(&call_data.id)?;
        let kind = call_data.r#type.into();

        self.apply_model_update(|project, changes| {
            project
                .new_control(changes, &view, id, call_data.x, call_data.y, kind)
                .map(Some)
        })
        .await
    }

    async fn clear_control(
        &mut self,
        call_data: project_manager::ClearControlUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(call_data.view_type, &call_data.view_id)?;
        let id = model::ControlId::new(&call_data.id)?;

        self.apply_model_update(|project, changes| project.clear_control(changes, &view, &id))
            .await
    }

    async fn rename_control(
        &mut self,
        call_data: project_manager::RenameControlUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(call_data.view_type, &call_data.view_id)?;
        let id = model::ControlId::new(&call_data.id)?;
        let new_id = model::ControlId::new(&call_data.new_id)?;

        self.apply_model_update(|project, changes| {
            project.rename_control(changes, &view, &id, &new_id)
        })
        .await
    }

    async fn clone_control(
        &mut self,
        call_data: project_manager::CloneControlUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let source = model::wire::view_key(call_data.view_type, &call_data.view_id)?;
        let target = model::wire::view_key(call_data.target_view_type, &call_data.target_view_id)?;
        let id = model::ControlId::new(&call_data.id)?;
        let new_id = model::ControlId::new(&call_data.new_id)?;

        self.apply_model_update(|project, changes| {
            project
                .clone_control(changes, &source, &id, &target, new_id)
                .map(Some)
        })
        .await
    }

    async fn set_control_properties(
        &mut self,
        call_data: project_manager::SetControlPropertiesUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(call_data.view_type, &call_data.view_id)?;
        let id = model::ControlId::new(&call_data.id)?;
        let properties = model::ControlProperties::try_from(call_data.properties)?;

        self.apply_model_update(|project, changes| {
            project.set_control_properties(changes, &view, id, properties)
        })
        .await
    }

    // -----------------------------------------------------------------------
    // Template instances
    // -----------------------------------------------------------------------

    async fn new_template_instance(
        &mut self,
        call_data: project_manager::NewTemplateInstanceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(call_data.view_type, &call_data.view_id)?;
        let id = model::TemplateInstanceId::new(&call_data.id)?;
        let template_id = model::TemplateId::new(&call_data.template_id)?;

        self.apply_model_update(|project, changes| {
            project
                .new_template_instance(changes, &view, id, template_id, call_data.x, call_data.y)
                .map(Some)
        })
        .await
    }

    async fn clear_template_instance(
        &mut self,
        call_data: project_manager::ClearTemplateInstanceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(call_data.view_type, &call_data.view_id)?;
        let id = model::TemplateInstanceId::new(&call_data.id)?;

        self.apply_model_update(|project, changes| {
            project.clear_template_instance(changes, &view, &id)
        })
        .await
    }

    async fn rename_template_instance(
        &mut self,
        call_data: project_manager::RenameTemplateInstanceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(call_data.view_type, &call_data.view_id)?;
        let id = model::TemplateInstanceId::new(&call_data.id)?;
        let new_id = model::TemplateInstanceId::new(&call_data.new_id)?;

        self.apply_model_update(|project, changes| {
            project.rename_template_instance(changes, &view, &id, &new_id)
        })
        .await
    }

    async fn clone_template_instance(
        &mut self,
        call_data: project_manager::CloneTemplateInstanceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(call_data.view_type, &call_data.view_id)?;
        let id = model::TemplateInstanceId::new(&call_data.id)?;
        let new_id = model::TemplateInstanceId::new(&call_data.new_id)?;

        self.apply_model_update(|project, changes| {
            project
                .clone_template_instance(changes, &view, &id, new_id)
                .map(Some)
        })
        .await
    }

    async fn move_template_instance(
        &mut self,
        call_data: project_manager::MoveTemplateInstanceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(call_data.view_type, &call_data.view_id)?;
        let id = model::TemplateInstanceId::new(&call_data.id)?;

        self.apply_model_update(|project, changes| {
            project
                .move_template_instance(changes, &view, id, call_data.x, call_data.y)
                .map(Some)
        })
        .await
    }

    async fn set_template_instance_template(
        &mut self,
        call_data: project_manager::SetTemplateInstanceTemplateUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(call_data.view_type, &call_data.view_id)?;
        let id = model::TemplateInstanceId::new(&call_data.id)?;
        let template_id = model::TemplateId::new(&call_data.template_id)?;

        self.apply_model_update(|project, changes| {
            project
                .set_template_instance_template(changes, &view, id, template_id)
                .map(Some)
        })
        .await
    }

    /// The awkward one: a binding targets either a component member, looked up
    /// in the project, or an export of the enclosing template, looked up in the
    /// view the instance sits in. The operation takes the view key and derives
    /// the owner from it, so there is no owner argument for a handler to get
    /// wrong.
    async fn set_template_instance_bindings(
        &mut self,
        call_data: project_manager::SetTemplateInstanceBindingsUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        let view = model::wire::view_key(call_data.view_type, &call_data.view_id)?;
        let id = model::TemplateInstanceId::new(&call_data.id)?;
        let bindings = model::wire::bindings(call_data.bindings)?;

        self.apply_model_update(|project, changes| {
            project
                .set_template_instance_bindings(changes, &view, id, bindings)
                .map(Some)
        })
        .await
    }
}
