use std::sync::Arc;

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{operation::ProjectOperation, window::WindowOperation},
};

pub trait ResourceOperation {
    /// Note: self.data() will not be modified by this function
    fn resource_initial_notifications(&mut self);

    fn resource_set(
        &mut self,
        id: String,
        resource: project_manager::UiResourceData,
    ) -> Result<(), ProjectManagerActorError>;
    fn resource_clear(&mut self, id: String) -> Result<(), ProjectManagerActorError>;
    fn resource_rename(
        &mut self,
        id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError>;
}

impl ResourceOperation for ProjectOperation {
    fn resource_initial_notifications(&mut self) {
        for (id, resource) in self.data().resources.clone().iter() {
            self.push_notification(project_manager::UpdateProjectNotification::SetUiResource(
                project_manager::SetUiResourceNotification {
                    id: id.clone(),
                    resource: resource.clone(),
                },
            ));
        }
    }

    fn resource_set(
        &mut self,
        id: String,
        resource: project_manager::UiResourceData,
    ) -> Result<(), ProjectManagerActorError> {
        self.check_id(&id)?;

        let resource = Arc::new(resource);
        let mut new_resources = self.data().resources.as_ref().clone();
        new_resources.insert(id.clone(), resource.clone());

        self.set_data(Arc::new(project_manager::UiProject {
            resources: Arc::new(new_resources),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(project_manager::UpdateProjectNotification::SetUiResource(
            project_manager::SetUiResourceNotification { id, resource },
        ));

        Ok(())
    }

    fn resource_clear(&mut self, id: String) -> Result<(), ProjectManagerActorError> {
        let mut new_resources = self.data().resources.as_ref().clone();
        if new_resources.remove(&id).is_none() {
            return Ok(());
        }

        self.set_data(Arc::new(project_manager::UiProject {
            resources: Arc::new(new_resources),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(project_manager::UpdateProjectNotification::ClearUiResource(
            project_manager::ClearUiResourceNotification { id: id.clone() },
        ));

        self.window_on_change_resource(&id, None)?;

        Ok(())
    }

    fn resource_rename(
        &mut self,
        id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.check_id(&new_id)?;

        if id == new_id {
            return Ok(());
        }

        if self.data().resources.contains_key(&new_id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Resource with id '{}' already exists",
                new_id
            )));
        }

        let mut new_resources = self.data().resources.as_ref().clone();

        let Some(resource) = new_resources.remove(&id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Resource with id '{}' not found",
                id
            )));
        };

        new_resources.insert(new_id.clone(), resource);

        self.set_data(Arc::new(project_manager::UiProject {
            resources: Arc::new(new_resources),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(
            project_manager::UpdateProjectNotification::RenameUiResource(
                project_manager::RenameUiResourceNotification {
                    id: id.clone(),
                    new_id: new_id.clone(),
                },
            ),
        );

        self.window_on_change_resource(&id, Some(&new_id))?;

        Ok(())
    }
}
