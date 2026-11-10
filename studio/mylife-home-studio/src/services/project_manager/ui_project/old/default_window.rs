use std::sync::Arc;

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError, ui_project::old::operation::ProjectOperation,
};

pub trait DefaultWindowOperation {
    /// Note: ctx.data() will not be modified by this function
    fn default_window_initial_notifications(&mut self);

    fn default_window_set(
        &mut self,
        default_window: project_manager::UiDefaultWindowData,
    ) -> Result<(), ProjectManagerActorError>;
    fn default_window_on_change_window(
        &mut self,
        window_id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError>;
}

impl DefaultWindowOperation for ProjectOperation {
    fn default_window_initial_notifications(&mut self) {
        self.push_notification(
            project_manager::UpdateProjectNotification::SetUiDefaultWindow(
                project_manager::SetUiDefaultWindowNotification {
                    default_window: self.data().default_window.clone(),
                },
            ),
        );
    }

    fn default_window_set(
        &mut self,
        default_window: project_manager::UiDefaultWindowData,
    ) -> Result<(), ProjectManagerActorError> {
        let mut new_map = self.data().default_window.0.clone();

        for (key, value) in default_window.0 {
            new_map.insert(key.clone(), value);
        }

        self.set_data(Arc::new(project_manager::UiProject {
            default_window: Arc::new(project_manager::UiDefaultWindowData(new_map)),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(
            project_manager::UpdateProjectNotification::SetUiDefaultWindow(
                project_manager::SetUiDefaultWindowNotification {
                    default_window: self.data().default_window.clone(),
                },
            ),
        );

        Ok(())
    }

    fn default_window_on_change_window(
        &mut self,
        window_id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError> {
        let mut new_map = self.data().default_window.0.clone();
        let mut changed = false;

        for value in new_map.values_mut() {
            if value.as_deref() == Some(window_id) {
                *value = new_id.map(str::to_owned);
                changed = true;
            }
        }

        if !changed {
            return Ok(());
        }

        self.set_data(Arc::new(project_manager::UiProject {
            default_window: Arc::new(project_manager::UiDefaultWindowData(new_map)),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(
            project_manager::UpdateProjectNotification::SetUiDefaultWindow(
                project_manager::SetUiDefaultWindowNotification {
                    default_window: self.data().default_window.clone(),
                },
            ),
        );

        Ok(())
    }
}
