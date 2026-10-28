use std::{collections::HashMap, format, mem, sync::Arc};

use studio_web_api::project_manager;

use crate::services::project_manager::ProjectManagerActorError;

#[derive(Debug)]
pub struct ProjectModel {
    data: Arc<project_manager::UiProject>,

    default_window: DefaultWindowModel,
    resources: ResourcesModel,
}

impl ProjectModel {
    pub fn new(data: &Arc<project_manager::UiProject>) -> Self {
        Self {
            data: data.clone(),

            default_window: DefaultWindowModel::new(&data.default_window),
            resources: ResourcesModel::new(&data.resources),
        }
    }

    pub fn initial_notifications(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
    ) {
        self.default_window.initial_notifications(notifications);
        /*

            notifier.notify({ operation: 'set-ui-default-window', defaultWindow: this.project.defaultWindow } as SetUiDefaultWindowNotification);
            notifier.notify({ operation: 'set-ui-component-data', components: this.project.components, plugins: this.project.plugins } as SetUiComponentDataNotification);
        */

        self.resources.initial_notifications(notifications);

        /*

            for (const [id, style] of Object.entries(this.project.styles)) {
              notifier.notify({ operation: 'set-ui-style', id, style } as SetUiStyleNotification);
            }

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
}

#[derive(Debug)]
struct DefaultWindowModel {
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

#[derive(Debug)]
struct ResourceModel {
    id: String,
    data: Arc<project_manager::UiResourceData>,
}

impl ResourceModel {
    pub fn new(id: &String, data: &Arc<project_manager::UiResourceData>) -> Self {
        Self {
            id: id.clone(),
            data: data.clone(),
        }
    }

    pub fn initial_notifications(
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

    pub fn data(&self) -> &Arc<project_manager::UiResourceData> {
        &self.data
    }

    pub fn rename(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        new_id: String,
    ) {
        let mut old_id = new_id;
        mem::swap(&mut old_id, &mut self.id);

        notifications.push(project_manager::UpdateProjectNotification::RenameUiResource(
            project_manager::RenameUiResourceNotification {
                id: old_id,
                new_id: self.id.clone(),
            },
        ));
    }

    pub fn set(
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
}

#[derive(Debug)]
struct ResourcesModel {
    data: Arc<HashMap<String, Arc<project_manager::UiResourceData>>>,
    resources: HashMap<String, ResourceModel>,
}

impl ResourcesModel {
    pub fn new(data: &Arc<HashMap<String, Arc<project_manager::UiResourceData>>>) -> Self {
        let mut resources = HashMap::new();

        for (id, resource_data) in data.as_ref() {
            resources.insert(id.clone(), ResourceModel::new(id, resource_data));
        }

        Self {
            data: data.clone(),
            resources,
        }
    }

    pub fn initial_notifications(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
    ) {
        for resource in self.resources.values() {
            resource.initial_notifications(notifications);
        }
    }

    pub fn data(&self) -> &Arc<HashMap<String, Arc<project_manager::UiResourceData>>> {
        &self.data
    }

    pub fn set(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        resource: project_manager::UiResourceData,
    ) -> Result<(), ProjectManagerActorError> {
        check_id(&id)?;

        let new_data = match self.resources.get_mut(&id) {
            Some(item) => {
                item.set(notifications, resource)?;
                item.data().clone()
            }
            None => {
                let new_item = ResourceModel::new(&id, &Arc::new(resource));
                let data = new_item.data().clone();
                new_item.initial_notifications(notifications);
                self.resources.insert(id.clone(), new_item);

                data
            }
        };

        // Rebuild the map
        let mut new_map = self.data.as_ref().clone();
        new_map.insert(id.clone(), new_data);

        self.data = Arc::new(new_map);

        Ok(())
    }

    pub fn clear(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<(), ProjectManagerActorError> {
        if self.resources.remove(&id).is_none() {
            return Ok(());
        }

        notifications.push(project_manager::UpdateProjectNotification::ClearUiResource(project_manager::ClearUiResourceNotification{id: id.clone()}));

        let mut new_map = self.data.as_ref().clone();
        new_map.remove(&id);
        self.data = Arc::new(new_map);

        Ok(())
    }

    pub fn rename(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        old_id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        check_id(&new_id)?;

        if self.resources.get(&new_id).is_some() {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Resource with id '{}' already exists",
                new_id
            )));
        }

        let Some(mut resource) = self.resources.remove(&old_id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Resource with id '{}' not found",
                old_id
            )));
        };

        let data = resource.data().clone();

        resource.rename(notifications, new_id.clone());
        self.resources.insert(new_id.clone(), resource);

        let mut new_map = self.data.as_ref().clone();
        new_map.remove(&old_id);
        new_map.insert(new_id, data);
        self.data = Arc::new(new_map);

        Ok(())
    }
}

fn check_id(id: &str) -> Result<(), ProjectManagerActorError> {
    if id.contains(':') {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Id '{}' contains forbidden character ':'",
            id
        )));
    }
    Ok(())
}
