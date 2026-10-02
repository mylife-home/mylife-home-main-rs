use async_trait::async_trait;
use studio_web_api::{project_manager, protocol};

use crate::{
    services::project_manager::{
        ProjectManagerActorError,
        opened_projects::{InitialEmitter, NotificationsEmitter, OpenedProject, TypedProjectData},
    },
    web::{NotifierManager, SessionEvent, SessionHandle},
};

/// Represents an opened core project
#[derive(Debug)]
pub struct CoreProject {
    name: String,
    notifiers: NotifierManager<project_manager::UpdateProjectNotification>,
    data: project_manager::CoreProject,
}

impl CoreProject {
    /// Opens a core project with the given data.
    pub fn open(
        name: &str,
        data: project_manager::CoreProject,
    ) -> Result<Self, ProjectManagerActorError> {
        Ok(Self {
            name: name.to_owned(),
            notifiers: NotifierManager::new("project-manager/opened-project"),
            data,
        })
    }
}

#[async_trait]
impl OpenedProject for CoreProject {
    fn r#type(&self) -> project_manager::ProjectType {
        project_manager::ProjectType::Core
    }

    fn rename(&mut self, new_name: &str) {
        self.name = new_name.to_owned();
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn reload(&mut self, project_data: TypedProjectData) {
        todo!()
    }

    fn session_event(&mut self, event: &SessionEvent) {
        todo!()
    }

    fn unused(&self) -> bool {
        self.notifiers.is_empty()
    }

    fn add_notifier(
        &mut self,
        session: SessionHandle,
    ) -> (protocol::NotifierId, Box<dyn NotificationsEmitter>) {
        let notifier = self.notifiers.create_notifier(session);

        // TODO: initial notifications
        let notifications = Vec::new();

        let notifier_id = protocol::NotifierId {
            notifier_id: notifier.notifier_id().to_owned(),
        };
        let notification_emitter = Box::new(InitialEmitter::new(notifier.clone(), notifications));

        (notifier_id, notification_emitter)
    }

    fn remove_notifier(&mut self, session: &SessionHandle, notifier_id: &str) -> bool {
        self.notifiers.remove_notifier(session, notifier_id)
    }

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
    > {
        let data = serde_json::from_value::<project_manager::CoreProjectCall>(data.0)?;

        todo!()
    }
}
