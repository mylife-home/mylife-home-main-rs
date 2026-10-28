use std::{mem, sync::Arc};

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError, ui_project::model::collection::CollectionItem,
};

#[derive(Debug)]
pub struct StyleModel {
    id: String,
    data: Arc<project_manager::UiStyleData>,
}

impl CollectionItem for StyleModel {
    type UiData = project_manager::UiStyleData;

    fn new(id: &String, data: &Arc<project_manager::UiStyleData>) -> Self {
        Self {
            id: id.clone(),
            data: data.clone(),
        }
    }

    fn initial_notifications(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
    ) {
        notifications.push(project_manager::UpdateProjectNotification::SetUiStyle(
            project_manager::SetUiStyleNotification {
                id: self.id.clone(),
                style: self.data.as_ref().clone(),
            },
        ));
    }

    fn data(&self) -> &Arc<project_manager::UiStyleData> {
        &self.data
    }

    fn rename(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        new_id: String,
    ) {
        let mut old_id = new_id;
        mem::swap(&mut old_id, &mut self.id);

        notifications.push(project_manager::UpdateProjectNotification::RenameUiStyle(
            project_manager::RenameUiStyleNotification {
                id: old_id,
                new_id: self.id.clone(),
            },
        ));
    }

    fn set(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        data: project_manager::UiStyleData,
    ) -> Result<(), ProjectManagerActorError> {
        self.data = Arc::new(data);

        notifications.push(project_manager::UpdateProjectNotification::SetUiStyle(
            project_manager::SetUiStyleNotification {
                id: self.id.clone(),
                style: self.data.as_ref().clone(),
            },
        ));

        Ok(())
    }

    fn clear(self, notifications: &mut Vec<project_manager::UpdateProjectNotification>) {
        notifications.push(project_manager::UpdateProjectNotification::ClearUiStyle(
            project_manager::ClearUiStyleNotification { id: self.id },
        ));
    }
}
