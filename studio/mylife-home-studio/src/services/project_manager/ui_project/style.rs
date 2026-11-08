use std::sync::Arc;

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{operation::ProjectOperation, window::WindowOperation},
};

pub trait StyleOperation {
    /// Note: self.data() will not be modified by this function
    fn style_initial_notifications(&mut self);

    fn style_set(
        &mut self,
        id: String,
        style: project_manager::UiStyleData,
    ) -> Result<(), ProjectManagerActorError>;
    fn style_clear(&mut self, id: String) -> Result<(), ProjectManagerActorError>;
    fn style_rename(&mut self, id: String, new_id: String) -> Result<(), ProjectManagerActorError>;
}

impl StyleOperation for ProjectOperation {
    fn style_initial_notifications(&mut self) {
        for (id, style) in self.data().styles.clone().iter() {
            self.push_notification(project_manager::UpdateProjectNotification::SetUiStyle(
                project_manager::SetUiStyleNotification {
                    id: id.clone(),
                    style: style.clone(),
                },
            ));
        }
    }

    fn style_set(
        &mut self,
        id: String,
        style: project_manager::UiStyleData,
    ) -> Result<(), ProjectManagerActorError> {
        self.check_id(&id)?;

        let style = Arc::new(style);
        let mut new_styles = self.data().styles.as_ref().clone();
        new_styles.insert(id.clone(), style.clone());

        self.set_data(Arc::new(project_manager::UiProject {
            styles: Arc::new(new_styles),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(project_manager::UpdateProjectNotification::SetUiStyle(
            project_manager::SetUiStyleNotification { id, style },
        ));

        Ok(())
    }

    fn style_clear(&mut self, id: String) -> Result<(), ProjectManagerActorError> {
        let mut new_styles = self.data().styles.as_ref().clone();
        if new_styles.remove(&id).is_none() {
            return Ok(());
        }

        self.set_data(Arc::new(project_manager::UiProject {
            styles: Arc::new(new_styles),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(project_manager::UpdateProjectNotification::ClearUiStyle(
            project_manager::ClearUiStyleNotification { id: id.clone() },
        ));

        self.window_on_change_style(&id, None)?;

        Ok(())
    }

    fn style_rename(&mut self, id: String, new_id: String) -> Result<(), ProjectManagerActorError> {
        self.check_id(&new_id)?;

        if id == new_id {
            return Ok(());
        }

        if self.data().styles.contains_key(&new_id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Style with id '{}' already exists",
                new_id
            )));
        }

        let mut new_styles = self.data().styles.as_ref().clone();

        let Some(style) = new_styles.remove(&id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Style with id '{}' not found",
                id
            )));
        };

        new_styles.insert(new_id.clone(), style);

        self.set_data(Arc::new(project_manager::UiProject {
            styles: Arc::new(new_styles),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(project_manager::UpdateProjectNotification::RenameUiStyle(
            project_manager::RenameUiStyleNotification {
                id: id.clone(),
                new_id: new_id.clone(),
            },
        ));

        self.window_on_change_style(&id, Some(&new_id))?;

        Ok(())
    }
}
