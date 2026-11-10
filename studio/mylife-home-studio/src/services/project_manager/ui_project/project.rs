use std::sync::Arc;

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::utils::{ArcMapOps, check_id},
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

    fn update(
        &self,
        mutator: impl FnOnce(&mut project_manager::UiProject) -> Result<(), ProjectManagerActorError>,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;
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

        let project = self.update(|project| {
            project.resources = project.resources.with_set(id.clone(), resource.clone())?;
            Ok(())
        })?;

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
        let project = self.update(|project| {
            project.resources = project.resources.with_cleared(&id)?;
            Ok(())
        })?;

        notifications.push(project_manager::UpdateProjectNotification::ClearUiResource(
            project_manager::ClearUiResourceNotification { id },
        ));

        // TODO: self.view_on_change_resource(&id, None)?;

        Ok(project)
    }

    fn rename_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let project = self.update(|project| {
            project.resources = project.resources.with_renamed(&id, &new_id)?;
            Ok(())
        })?;

        notifications.push(project_manager::UpdateProjectNotification::RenameUiResource(
            project_manager::RenameUiResourceNotification { id, new_id },
        ));

        // TODO: self.view_on_change_resource(&id, Some(&new_id))?;

        Ok(project)
    }

    fn update(
        &self,
        mutator: impl FnOnce(&mut project_manager::UiProject) -> Result<(), ProjectManagerActorError>,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let mut project = self.as_ref().clone();
        mutator(&mut project)?;
        Ok(Arc::new(project))
    }
}
