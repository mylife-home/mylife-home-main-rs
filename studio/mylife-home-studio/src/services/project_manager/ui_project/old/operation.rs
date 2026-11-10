use std::sync::Arc;

use studio_web_api::project_manager;

use crate::services::project_manager::ProjectManagerActorError;

pub struct ProjectOperation {
    notifications: Vec<project_manager::UpdateProjectNotification>,
    data: Arc<project_manager::UiProject>,
}

impl ProjectOperation {
    pub fn new(data: &Arc<project_manager::UiProject>) -> Self {
        Self {
            notifications: Vec::new(),
            data: data.clone(),
        }
    }

    pub fn output(
        self,
    ) -> (
        Vec<project_manager::UpdateProjectNotification>,
        Arc<project_manager::UiProject>,
    ) {
        (self.notifications, self.data)
    }

    pub fn push_notification(&mut self, notification: project_manager::UpdateProjectNotification) {
        self.notifications.push(notification);
    }

    pub fn data(&self) -> &Arc<project_manager::UiProject> {
        &self.data
    }

    pub fn set_data(&mut self, data: Arc<project_manager::UiProject>) {
        self.data = data;
    }

    pub fn check_id(&self, id: &str) -> Result<(), ProjectManagerActorError> {
        if id.trim().is_empty() {
            return Err(ProjectManagerActorError::DataProcessingError(String::from(
                "Id is empty",
            )));
        }

        if id.contains(':') {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Id '{}' contains forbidden character ':'",
                id
            )));
        }
        Ok(())
    }
}
