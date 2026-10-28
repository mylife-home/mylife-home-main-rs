use std::{format, sync::Arc};

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::model::{
        collection::CollectionModel, default_window::DefaultWindowModel, resource::ResourceModel,
        style::StyleModel,
    },
};

mod collection;
mod default_window;
mod resource;
mod style;

#[derive(Debug)]
pub struct ProjectModel {
    data: Arc<project_manager::UiProject>,

    default_window: DefaultWindowModel,
    resources: CollectionModel<ResourceModel>,
    styles: CollectionModel<StyleModel>,
}

impl ProjectModel {
    pub fn new(data: &Arc<project_manager::UiProject>) -> Self {
        Self {
            data: data.clone(),

            default_window: DefaultWindowModel::new(&data.default_window),
            resources: CollectionModel::new(&data.resources),
            styles: CollectionModel::new(&data.styles),
        }
    }

    pub fn initial_notifications(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
    ) {
        self.default_window.initial_notifications(notifications);

        /*
            notifier.notify({ operation: 'set-ui-component-data', components: this.project.components, plugins: this.project.plugins } as SetUiComponentDataNotification);
        */

        self.resources.initial_notifications(notifications);
        self.styles.initial_notifications(notifications);

        /*
            for (const [id, window] of Object.entries(this.project.windows)) {
              notifier.notify({ operation: 'set-ui-window', id, window } as SetUiWindowNotification);
            }

            for (const [id, template] of Object.entries(this.project.templates)) {
              notifier.notify({ operation: 'set-ui-template', id, template } as SetUiTemplateNotification);
            }
        */
    }

    pub fn data(&self) -> &Arc<project_manager::UiProject> {
        &self.data
    }

    pub fn set_default_window(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        data: project_manager::UiDefaultWindowData,
    ) -> Result<(), ProjectManagerActorError> {
        self.default_window.set(notifications, data)?;

        self.data = Arc::new(project_manager::UiProject {
            default_window: self.default_window.data().clone(),
            ..(*self.data).clone()
        });

        Ok(())
    }

    pub fn set_resource(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        resource: project_manager::UiResourceData,
    ) -> Result<(), ProjectManagerActorError> {
        self.resources.set(notifications, id, resource)?;

        self.data = Arc::new(project_manager::UiProject {
            resources: self.resources.data().clone(),
            ..(*self.data).clone()
        });

        Ok(())
    }

    pub fn clear_resource(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.resources.clear(notifications, id)?;

        // TODO: impacts

        self.data = Arc::new(project_manager::UiProject {
            resources: self.resources.data().clone(),
            ..(*self.data).clone()
        });

        Ok(())
    }

    pub fn rename_resource(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        old_id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.resources.rename(notifications, old_id, new_id)?;

        // TODO: impacts

        self.data = Arc::new(project_manager::UiProject {
            resources: self.resources.data().clone(),
            ..(*self.data).clone()
        });

        Ok(())
    }

    pub fn set_style(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        style: project_manager::UiStyleData,
    ) -> Result<(), ProjectManagerActorError> {
        self.styles.set(notifications, id, style)?;

        self.data = Arc::new(project_manager::UiProject {
            styles: self.styles.data().clone(),
            ..(*self.data).clone()
        });

        Ok(())
    }

    pub fn clear_style(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.styles.clear(notifications, id)?;

        // TODO: impacts

        self.data = Arc::new(project_manager::UiProject {
            styles: self.styles.data().clone(),
            ..(*self.data).clone()
        });

        Ok(())
    }

    pub fn rename_style(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        old_id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.styles.rename(notifications, old_id, new_id)?;

        // TODO: impacts

        self.data = Arc::new(project_manager::UiProject {
            styles: self.styles.data().clone(),
            ..(*self.data).clone()
        });

        Ok(())
    }
}
