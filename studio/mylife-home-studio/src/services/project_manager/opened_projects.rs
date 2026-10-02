// project opened once
// each opened tab in the ui has its own notifier
// project reloaded when changed externaly (+'reset' notification)
// opened project can add/remove notifiers (for new open/close/session close)
// when an opened project has no notifier anymore, it is closed (removed from the opened projects list)
// project can be renamed while opened

use std::{collections::HashMap, fmt::Debug, format, sync::Arc, todo};

use async_trait::async_trait;
use studio_web_api::{project_manager, protocol};

use crate::{
    services::project_manager::ProjectManagerActorError,
    web::{Notifier, NotifierManager, SessionEvent, SessionHandle, SessionId},
};

/// Represents an opened project within the system.
#[async_trait]
pub trait OpenedProject: Debug + Send + Sync {
    /// Returns the type of the opened project.
    fn r#type(&self) -> project_manager::ProjectType;

    /// Renames the opened project to the specified new name.
    fn rename(&mut self, new_name: &str);

    /// Returns the name of the opened project.
    fn name(&self) -> &str;

    /// Reloads the opened project, typically used when the project has been updated externally.
    fn reload(&mut self, project_data: TypedProjectData);

    /// Handles a session event for the opened project.
    fn session_event(&mut self, event: &SessionEvent);

    /// Determines whether the opened project is currently unused (no session is using it).
    fn unused(&self) -> bool;

    /// Adds a notifier for the specified session to the opened project.
    fn add_notifier(
        &mut self,
        session: SessionHandle,
    ) -> (protocol::NotifierId, Box<dyn NotificationsEmitter>);

    /// Removes the notifier for the specified session and notifier ID from the opened project.
    fn remove_notifier(&mut self, session: &SessionHandle, notifier_id: &str) -> bool;

    /// Handles a project call for the opened project.
    ///
    /// Returns the result of the project call and optionally updated project data (if project state changed).
    async fn call(
        &mut self,
        data: project_manager::ProjectCall,
        session: &SessionHandle,
        notifier_id: &str,
    ) -> Result<
        (
            project_manager::ProjectCallResult,
            Box<dyn NotificationsEmitter>,
            Option<TypedProjectData>,
        ),
        ProjectManagerActorError,
    >;
}

/// Represents an entity capable of emitting notifications for opened projects.
pub trait NotificationsEmitter: Debug + Send + Sync {
    /// Emits a notification for the opened project.
    fn emit_notifications(&mut self);
}

// TODO: Move to projects
#[derive(Debug)]
struct InitialEmitter {
    notifier: Notifier<project_manager::UpdateProjectNotification>,
    notifications: Vec<project_manager::UpdateProjectNotification>,
}

impl NotificationsEmitter for InitialEmitter {
    fn emit_notifications(&mut self) {
        for notification in self.notifications.drain(..) {
            self.notifier.notify(&notification);
        }
    }
}

// TODO: Move to projects
#[derive(Debug)]
struct BroadcastEmitter {
    notifiers: Arc<NotifierManager<project_manager::UpdateProjectNotification>>,
    notifications: Vec<project_manager::UpdateProjectNotification>,
}

impl NotificationsEmitter for BroadcastEmitter {
    fn emit_notifications(&mut self) {
        for notification in self.notifications.drain(..) {
            self.notifiers.notify_all(&notification);
        }
    }
}

#[derive(Debug, Clone)]
pub enum TypedProjectData {
    Core(project_manager::CoreProject),
    Ui(project_manager::UiProject),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct OpenedProjectId(usize);

/// Manages the collection of currently opened projects.
#[derive(Debug)]
pub struct OpenedProjects {
    /// The next available ID for a newly opened project.
    next_id: usize,

    /// The collection of currently opened projects, keyed by their unique ID.
    projects: HashMap<OpenedProjectId, Box<dyn OpenedProject>>,

    /// Maps project names to their corresponding opened project IDs.
    names: HashMap<String, OpenedProjectId>,

    /// Maps session and project name pairs (what the clients provide) to their corresponding opened project IDs.
    notifiers: HashMap<(SessionId, String), OpenedProjectId>,
}

impl OpenedProjects {
    /// Creates a new instance of `OpenedProjects`.
    pub fn new() -> Self {
        Self {
            next_id: 1,
            projects: HashMap::new(),
            names: HashMap::new(),
            notifiers: HashMap::new(),
        }
    }

    /// Reloads the project with the specified type and ID if it has been updated externally.
    pub fn reload_project(&mut self, id: &str, project_data: TypedProjectData) {
        let ty = match &project_data {
            TypedProjectData::Core(_) => project_manager::ProjectType::Core,
            TypedProjectData::Ui(_) => project_manager::ProjectType::Ui,
        };

        let Some(&project_id) = self.names.get(&Self::make_name(ty, id)) else {
            // not opened
            return;
        };

        let project = self
            .projects
            .get_mut(&project_id)
            .expect("Project should exist");

        project.reload(project_data);
    }

    /// Handles a session event for the opened projects.
    pub fn session_event(&mut self, event: &SessionEvent) {
        for project_id in self.projects.keys().cloned().collect::<Vec<_>>() {
            let project = self
                .projects
                .get_mut(&project_id)
                .expect("Project should exist");
            project.session_event(event);

            self.check_project_unused(project_id);
        }
    }

    /// Opens a project of the specified type and name.
    pub fn open_project(
        &mut self,
        session: &SessionHandle,
        project_data: TypedProjectData,
        name: &str,
    ) -> Result<(protocol::NotifierId, Box<dyn NotificationsEmitter>), ProjectManagerActorError>
    {
        let ty = match project_data {
            TypedProjectData::Core(_) => project_manager::ProjectType::Core,
            TypedProjectData::Ui(_) => project_manager::ProjectType::Ui,
        };

        let project_name = Self::make_name(ty, name);

        let project_id = match self.names.get(&project_name) {
            Some(&project_id) => project_id,
            None => {
                let project = self.do_open_project(name, project_data)?;

                let id = self.make_id();
                self.projects.insert(id, project);
                self.names.insert(project_name, id);

                id
            }
        };

        let project = self
            .projects
            .get_mut(&project_id)
            .expect("Project should exist");
        let (notifier, notifications_emitter) = project.add_notifier(session.clone());

        Ok((notifier, notifications_emitter))
    }

    fn make_id(&mut self) -> OpenedProjectId {
        let id = self.next_id;
        self.next_id += 1;
        OpenedProjectId(id)
    }

    fn do_open_project(
        &mut self,
        name: &str,
        project_data: TypedProjectData,
    ) -> Result<Box<dyn OpenedProject>, ProjectManagerActorError> {
        todo!()
    }

    /// Closes the project with the specified ID.
    pub fn close_project(
        &mut self,
        session: &SessionHandle,
        id: protocol::NotifierId,
    ) -> Result<(), ProjectManagerActorError> {
        let key = (session.id(), id.notifier_id.clone());

        let Some(&project_id) = self.notifiers.get(&key) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Project with notifier ID {:?} not found for session {:?}",
                id.notifier_id,
                session.id()
            )));
        };

        let project = self
            .projects
            .get_mut(&project_id)
            .expect("Project should exist");

        // Map key lookup success so the notifier must exist for the session
        project.remove_notifier(session, &id.notifier_id);
        self.notifiers.remove(&key);

        self.check_project_unused(project_id);

        Ok(())
    }

    /// Calls a project with the specified ID.
    pub async fn call_project(
        &mut self,
        session: &SessionHandle,
        id: protocol::NotifierId,
        data: project_manager::ProjectCall,
    ) -> Result<
        (
            project_manager::ProjectCallResult,
            Box<dyn NotificationsEmitter>,
            Option<(String, TypedProjectData)>, // if changed
        ),
        ProjectManagerActorError,
    > {
        let key = (session.id(), id.notifier_id.clone());

        let Some(&project_id) = self.notifiers.get(&key) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Project with notifier ID {:?} not found for session {:?}",
                id.notifier_id,
                session.id()
            )));
        };

        let project = self
            .projects
            .get_mut(&project_id)
            .expect("Project should exist");

        let (data, notifications_emitter, project_data) =
            project.call(data, session, &id.notifier_id).await?;

        let project_data =
            project_data.map(|project_data| (project.name().to_owned(), project_data));

        Ok((data, notifications_emitter, project_data))
    }

    /// Renames an opened project from `old_name` to `new_name`.
    pub fn rename_project(
        &mut self,
        ty: project_manager::ProjectType,
        old_name: &str,
        new_name: &str,
    ) {
        let old_id = Self::make_name(ty, old_name);
        let Some(&project_id) = self.names.get(&old_id) else {
            // not opened
            return;
        };

        let project = self
            .projects
            .get_mut(&project_id)
            .expect("Project should exist");
        project.rename(new_name);

        self.names.remove(&old_id);
        self.names.insert(Self::make_name(ty, new_name), project_id);
    }

    fn make_name(ty: project_manager::ProjectType, name: &str) -> String {
        let ty_str = match ty {
            project_manager::ProjectType::Core => "core",
            project_manager::ProjectType::Ui => "ui",
        };

        format!("{}:{}", ty_str, name)
    }

    fn check_project_unused(&mut self, project_id: OpenedProjectId) {
        let project = self
            .projects
            .get(&project_id)
            .expect("Project should exist");

        if !project.unused() {
            return;
        }

        self.names
            .remove(&Self::make_name(project.r#type(), project.name()));
        self.projects.remove(&project_id);
    }
}
