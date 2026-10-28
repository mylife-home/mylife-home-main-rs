use std::{mem, sync::Arc};

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError, ui_project::model::collection::CollectionItem,
};

#[derive(Debug)]
pub struct ResourceModel {
    id: String,
    data: Arc<project_manager::UiResourceData>,
}

impl CollectionItem for ResourceModel {
    type UiData = project_manager::UiResourceData;

    fn new(id: &String, data: &Arc<project_manager::UiResourceData>) -> Self {
        Self {
            id: id.clone(),
            data: data.clone(),
        }
    }

    fn initial_notifications(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
    ) {
        notifications.push(project_manager::UpdateProjectNotification::SetUiResource(
            project_manager::SetUiResourceNotification {
                id: self.id.clone(),
                resource: self.data.as_ref().clone(),
            },
        ));
    }

    fn data(&self) -> &Arc<project_manager::UiResourceData> {
        &self.data
    }

    fn rename(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        new_id: String,
    ) {
        let mut old_id = new_id;
        mem::swap(&mut old_id, &mut self.id);

        notifications.push(
            project_manager::UpdateProjectNotification::RenameUiResource(
                project_manager::RenameUiResourceNotification {
                    id: old_id,
                    new_id: self.id.clone(),
                },
            ),
        );
    }

    fn set(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        data: project_manager::UiResourceData,
    ) -> Result<(), ProjectManagerActorError> {
        self.data = Arc::new(data);

        notifications.push(project_manager::UpdateProjectNotification::SetUiResource(
            project_manager::SetUiResourceNotification {
                id: self.id.clone(),
                resource: self.data.as_ref().clone(),
            },
        ));

        Ok(())
    }

    fn clear(self, notifications: &mut Vec<project_manager::UpdateProjectNotification>) {
        notifications.push(project_manager::UpdateProjectNotification::ClearUiResource(
            project_manager::ClearUiResourceNotification { id: self.id },
        ));
    }
}
