use std::sync::Arc;

use studio_web_api::project_manager;

use crate::services::project_manager::ProjectManagerActorError;

#[derive(Debug)]
pub struct DefaultWindowModel {
    data: Arc<project_manager::UiDefaultWindowData>,
}

impl DefaultWindowModel {
    pub fn new(data: &Arc<project_manager::UiDefaultWindowData>) -> Self {
        Self { data: data.clone() }
    }

    pub fn initial_notifications(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
    ) {
        notifications.push(
            project_manager::UpdateProjectNotification::SetUiDefaultWindow(
                project_manager::SetUiDefaultWindowNotification {
                    default_window: self.data.as_ref().clone(),
                },
            ),
        );
    }

    pub fn data(&self) -> &Arc<project_manager::UiDefaultWindowData> {
        &self.data
    }

    pub fn set(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        data: project_manager::UiDefaultWindowData,
    ) -> Result<(), ProjectManagerActorError> {
        let mut new_map = self.data.0.clone();

        for (key, value) in data.0 {
            new_map.insert(key.clone(), value);
        }

        self.data = Arc::new(project_manager::UiDefaultWindowData(new_map));

        notifications.push(
            project_manager::UpdateProjectNotification::SetUiDefaultWindow(
                project_manager::SetUiDefaultWindowNotification {
                    default_window: self.data.as_ref().clone(),
                },
            ),
        );

        Ok(())
    }
}
