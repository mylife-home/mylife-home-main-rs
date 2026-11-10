use std::{sync::Arc, todo};

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{
        utils::{ArcMapOps, check_id},
        window::WindowOps,
    },
};

pub trait ProjectOps {
    fn set_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        resource: project_manager::UiResourceData,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn clear_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn rename_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn set_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        style: project_manager::UiStyleData,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn clear_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn rename_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn new_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn clear_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn rename_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn clone_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn set_window_properties(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        properties: project_manager::UiWindowProperties,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn on_change_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError>;

    fn on_change_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError>;

    fn on_change_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError>;
}

impl ProjectOps for Arc<project_manager::UiProject> {
    fn set_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        resource: project_manager::UiResourceData,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        check_id(&id)?;
        let resource = Arc::new(resource);

        let mut project = self.as_ref().clone();
        project.resources = project.resources.with_set(id.clone(), resource.clone())?;
        let project = Arc::new(project);

        notifications.push(project_manager::UpdateProjectNotification::SetUiResource(
            project_manager::SetUiResourceNotification { id, resource },
        ));

        Ok(project)
    }

    fn clear_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let mut project = self.as_ref().clone();
        project.resources = project.resources.with_cleared(&id)?;
        let project = Arc::new(project);

        notifications.push(project_manager::UpdateProjectNotification::ClearUiResource(
            project_manager::ClearUiResourceNotification { id: id.clone() },
        ));

        self.on_change_resource(notifications, &id, None)?;

        Ok(project)
    }

    fn rename_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let mut project = self.as_ref().clone();
        project.resources = project.resources.with_renamed(&id, &new_id)?;
        let project = Arc::new(project);

        notifications.push(
            project_manager::UpdateProjectNotification::RenameUiResource(
                project_manager::RenameUiResourceNotification {
                    id: id.clone(),
                    new_id: new_id.clone(),
                },
            ),
        );

        self.on_change_resource(notifications, &id, Some(&new_id))?;

        Ok(project)
    }

    fn set_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        style: project_manager::UiStyleData,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        check_id(&id)?;
        let style = Arc::new(style);

        let mut project = self.as_ref().clone();
        project.styles = project.styles.with_set(id.clone(), style.clone())?;
        let project = Arc::new(project);

        notifications.push(project_manager::UpdateProjectNotification::SetUiStyle(
            project_manager::SetUiStyleNotification { id, style },
        ));

        Ok(project)
    }

    fn clear_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let mut project = self.as_ref().clone();
        project.styles = project.styles.with_cleared(&id)?;
        let project = Arc::new(project);

        notifications.push(project_manager::UpdateProjectNotification::ClearUiStyle(
            project_manager::ClearUiStyleNotification { id: id.clone() },
        ));

        self.on_change_style(notifications, &id, None)?;

        Ok(project)
    }

    fn rename_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let mut project = self.as_ref().clone();
        project.styles = project.styles.with_renamed(&id, &new_id)?;
        let project = Arc::new(project);

        notifications.push(project_manager::UpdateProjectNotification::RenameUiStyle(
            project_manager::RenameUiStyleNotification {
                id: id.clone(),
                new_id: new_id.clone(),
            },
        ));

        self.on_change_style(notifications, &id, Some(&new_id))?;

        Ok(project)
    }

    fn new_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let window = project_manager::UiWindowData::new();

        let mut project = self.as_ref().clone();
        project.windows = project.windows.with_inserted(id.clone(), window.clone())?;
        let project = Arc::new(project);

        notifications.push(project_manager::UpdateProjectNotification::SetUiWindow(
            project_manager::SetUiWindowNotification { id, window },
        ));

        Ok(project)
    }

    fn clear_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let mut project = self.as_ref().clone();
        project.windows = project.windows.with_cleared(&id)?;
        let project = Arc::new(project);

        notifications.push(project_manager::UpdateProjectNotification::ClearUiWindow(
            project_manager::ClearUiWindowNotification { id: id.clone() },
        ));

        self.on_change_window(notifications, &id, None)?;

        Ok(project)
    }

    fn rename_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let mut project = self.as_ref().clone();
        project.windows = project.windows.with_renamed(&id, &new_id)?;
        let project = Arc::new(project);

        notifications.push(project_manager::UpdateProjectNotification::RenameUiWindow(
            project_manager::RenameUiWindowNotification {
                id: id.clone(),
                new_id: new_id.clone(),
            },
        ));

        self.on_change_window(notifications, &id, Some(&new_id))?;

        Ok(project)
    }

    fn clone_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let window = self.windows.get_required(&id)?.clone();

        let mut project = self.as_ref().clone();
        project.windows = project
            .windows
            .with_inserted(new_id.clone(), window.clone())?;
        let project = Arc::new(project);

        notifications.push(project_manager::UpdateProjectNotification::SetUiWindow(
            project_manager::SetUiWindowNotification { id: new_id, window },
        ));

        Ok(project)
    }

    fn set_window_properties(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        properties: project_manager::UiWindowProperties,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let mut project = self.as_ref().clone();
        let window = project.windows.get_required(&id)?;
        let window = window.set_properties(properties);
        project.windows = project.windows.with_updated(id.clone(), window.clone())?;
        let project = Arc::new(project);

        notifications.push(project_manager::UpdateProjectNotification::SetUiWindow(
            project_manager::SetUiWindowNotification { id, window },
        ));

        Ok(project)
    }

    fn on_change_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }


    fn on_change_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    fn on_change_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }
}
