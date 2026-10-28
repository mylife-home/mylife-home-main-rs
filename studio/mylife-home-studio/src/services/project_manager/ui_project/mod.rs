use std::{mem, sync::Arc, todo};

use common::utils::actors::{CallError, SpawnExt};
use serde::Serialize;
use std::collections::HashMap;
use studio_web_api::project_manager;

use crate::{
    services::project_manager::{InitialEmitter, ProjectManagerActorError},
    utils::fs_collection::{self, FsCollectionHandle, Update},
    web::{NotifierManager, SessionEvent, SessionHandle},
};

use kameo::{error::Infallible, message, prelude::*};

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

        let actor = UiProject::spawn_unbounded(UiProjectArgs {
            name,
            data,
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
struct UiProjectArgs {
    name: String,
    data: Arc<project_manager::UiProject>,
    fs_collection: FsCollectionHandle<project_manager::UiProject>,
}

/// Represents an opened ui project
#[derive(Debug)]
struct UiProject {
    name: String,
    notifiers: NotifierManager<project_manager::UpdateProjectNotification>,
    model: ProjectModel,
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
            model: ProjectModel::new(&args.data),
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
        let notifier = self.notifiers.create_notifier(msg.session);

        // TODO: initial notifications
        let mut notifications = Vec::new();

        notifications.push(
            project_manager::UpdateProjectNotification::SetUiDefaultWindow(
                project_manager::SetUiDefaultWindowNotification {
                    default_window: self.model.default_window.data.as_ref().clone(),
                },
            ),
        );
        /*

            notifier.notify({ operation: 'set-ui-default-window', defaultWindow: this.project.defaultWindow } as SetUiDefaultWindowNotification);
            notifier.notify({ operation: 'set-ui-component-data', components: this.project.components, plugins: this.project.plugins } as SetUiComponentDataNotification);

            for (const [id, resource] of Object.entries(this.project.resources)) {
              notifier.notify({ operation: 'set-ui-resource', id, resource } as SetUiResourceNotification);
            }

            for (const [id, style] of Object.entries(this.project.styles)) {
              notifier.notify({ operation: 'set-ui-style', id, style } as SetUiStyleNotification);
            }

            for (const [id, window] of Object.entries(this.project.windows)) {
              notifier.notify({ operation: 'set-ui-window', id, window } as SetUiWindowNotification);
            }

            for (const [id, template] of Object.entries(this.project.templates)) {
              notifier.notify({ operation: 'set-ui-template', id, template } as SetUiTemplateNotification);
            }
        */
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
struct Call {
    session: SessionHandle,
    notifier_id: String,
    call_data: project_manager::ProjectCall,
}

impl message::Message<Call> for UiProject {
    type Reply = Result<project_manager::ProjectCallResult, ProjectManagerActorError>;

    async fn handle(&mut self, msg: Call, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
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
                if Arc::ptr_eq(&self.model.data, &value) {
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
    fn reload(&mut self, value: Arc<project_manager::UiProject>) {
        self.model = ProjectModel::new(&value);
        // TODO: notify
    }

    async fn update(&mut self) -> Result<(), ProjectManagerActorError> {
        self.fs_collection
            .update(self.name.clone(), self.model.data.clone())
            .await?;
        Ok(())
    }

    async fn validate(
        &mut self,
    ) -> Result<project_manager::ValidateUiProjectCallResult, ProjectManagerActorError> {
        todo!()
    }

    async fn refresh_components_from_online(
        &mut self,
    ) -> Result<project_manager::RefreshComponentsUiProjectCallResult, ProjectManagerActorError>
    {
        todo!()
    }

    async fn refresh_components_from_project(
        &mut self,
        call_data: project_manager::RefreshComponentsFromProjectUiProjectCall,
    ) -> Result<project_manager::RefreshComponentsUiProjectCallResult, ProjectManagerActorError>
    {
        todo!()
    }

    async fn apply_refresh_components(
        &mut self,
        call_data: project_manager::ApplyRefreshComponentsUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn deploy(
        &mut self,
    ) -> Result<project_manager::DeployUiProjectCallResult, ProjectManagerActorError> {
        todo!()
    }

    async fn set_default_window(
        &mut self,
        call_data: project_manager::SetDefaultWindowUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        self.model.set_default_window(call_data.default_window);
        self.notify_all_default_window();
        self.update().await
    }

    async fn set_resource(
        &mut self,
        call_data: project_manager::SetResourceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn clear_resource(
        &mut self,
        call_data: project_manager::ClearResourceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn rename_resource(
        &mut self,
        call_data: project_manager::RenameResourceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn set_style(
        &mut self,
        call_data: project_manager::SetStyleUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn clear_style(
        &mut self,
        call_data: project_manager::ClearStyleUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn rename_style(
        &mut self,
        call_data: project_manager::RenameStyleUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn new_window(
        &mut self,
        call_data: project_manager::NewWindowUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn clear_window(
        &mut self,
        call_data: project_manager::ClearWindowUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn rename_window(
        &mut self,
        call_data: project_manager::RenameWindowUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn clone_window(
        &mut self,
        call_data: project_manager::CloneWindowUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn set_window_properties(
        &mut self,
        call_data: project_manager::SetWindowPropertiesUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn new_template(
        &mut self,
        call_data: project_manager::NewTemplateUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn clear_template(
        &mut self,
        call_data: project_manager::ClearTemplateUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn rename_template(
        &mut self,
        call_data: project_manager::RenameTemplateUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn clone_template(
        &mut self,
        call_data: project_manager::CloneTemplateUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn set_template_properties(
        &mut self,
        call_data: project_manager::SetTemplatePropertiesUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn set_template_export(
        &mut self,
        call_data: project_manager::SetTemplateExportUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn clear_template_export(
        &mut self,
        call_data: project_manager::ClearTemplateExportUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn set_template_bulk_patterns(
        &mut self,
        call_data: project_manager::SetTemplateBulkPatternsUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn new_control(
        &mut self,
        call_data: project_manager::NewControlUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn clear_control(
        &mut self,
        call_data: project_manager::ClearControlUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn rename_control(
        &mut self,
        call_data: project_manager::RenameControlUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn clone_control(
        &mut self,
        call_data: project_manager::CloneControlUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn set_control_properties(
        &mut self,
        call_data: project_manager::SetControlPropertiesUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn new_template_instance(
        &mut self,
        call_data: project_manager::NewTemplateInstanceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn clear_template_instance(
        &mut self,
        call_data: project_manager::ClearTemplateInstanceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn rename_template_instance(
        &mut self,
        call_data: project_manager::RenameTemplateInstanceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn clone_template_instance(
        &mut self,
        call_data: project_manager::CloneTemplateInstanceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn move_template_instance(
        &mut self,
        call_data: project_manager::MoveTemplateInstanceUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn set_template_instance_template(
        &mut self,
        call_data: project_manager::SetTemplateInstanceTemplateUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    async fn set_template_instance_bindings(
        &mut self,
        call_data: project_manager::SetTemplateInstanceBindingsUiProjectCall,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    fn notify_all_default_window(&mut self) {
        self.notifiers.notify_all(
            &project_manager::UpdateProjectNotification::SetUiDefaultWindow(
                project_manager::SetUiDefaultWindowNotification {
                    default_window: self.model.default_window.data.as_ref().clone(),
                },
            ),
        );
    }
}

#[derive(Debug)]
struct ProjectModel {
    pub data: Arc<project_manager::UiProject>,
    pub default_window: DefaultWindowModel,
}

impl ProjectModel {
    pub fn new(data: &Arc<project_manager::UiProject>) -> Self {
        Self {
            default_window: DefaultWindowModel::new(&data.default_window),
            data: data.clone(),
        }
    }

    pub fn set_default_window(&mut self, data: project_manager::UiDefaultWindowData) {
        self.default_window.set(data);

        self.data = Arc::new(project_manager::UiProject {
            default_window: self.default_window.data.clone(),
            ..(*self.data).clone()
        });
    }
}

#[derive(Debug)]
struct DefaultWindowModel {
    pub data: Arc<project_manager::UiDefaultWindowData>,
}

impl DefaultWindowModel {
    pub fn new(data: &Arc<project_manager::UiDefaultWindowData>) -> Self {
        Self { data: data.clone() }
    }

    pub fn set(&mut self, data: project_manager::UiDefaultWindowData) {
        let mut new_map = self.data.0.clone();

        for (key, value) in data.0 {
            new_map.insert(key.clone(), value);
        }

        self.data = Arc::new(project_manager::UiDefaultWindowData(new_map));
    }
}
