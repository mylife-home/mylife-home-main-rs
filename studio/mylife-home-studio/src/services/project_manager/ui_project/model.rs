use std::{collections::HashMap, format, mem, sync::Arc};

use studio_web_api::project_manager;

use crate::services::project_manager::ProjectManagerActorError;

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

trait CollectionItem {
    type UiData;

    fn new(id: &String, data: &Arc<Self::UiData>) -> Self;

    fn initial_notifications(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
    );

    fn data(&self) -> &Arc<Self::UiData>;

    fn set(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        data: Self::UiData,
    ) -> Result<(), ProjectManagerActorError>;

    fn rename(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        new_id: String,
    );

    fn clear(self, notifications: &mut Vec<project_manager::UpdateProjectNotification>);
}

#[derive(Debug)]
struct CollectionModel<Item: CollectionItem> {
    data: Arc<HashMap<String, Arc<Item::UiData>>>,
    items: HashMap<String, Item>,
}

impl<Item: CollectionItem> CollectionModel<Item> {
    pub fn new(data: &Arc<HashMap<String, Arc<Item::UiData>>>) -> Self {
        let mut items = HashMap::new();

        for (id, item_data) in data.as_ref() {
            items.insert(id.clone(), Item::new(id, item_data));
        }

        Self {
            data: data.clone(),
            items,
        }
    }

    pub fn initial_notifications(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
    ) {
        for resource in self.items.values() {
            resource.initial_notifications(notifications);
        }
    }

    pub fn data(&self) -> &Arc<HashMap<String, Arc<Item::UiData>>> {
        &self.data
    }

    pub fn set(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        data: Item::UiData,
    ) -> Result<(), ProjectManagerActorError> {
        check_id(&id)?;

        let new_data = match self.items.get_mut(&id) {
            Some(item) => {
                item.set(notifications, data)?;
                item.data().clone()
            }
            None => {
                let new_item = Item::new(&id, &Arc::new(data));
                let data = new_item.data().clone();
                new_item.initial_notifications(notifications);
                self.items.insert(id.clone(), new_item);

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
        let Some(item) = self.items.remove(&id) else {
            return Ok(());
        };

        item.clear(notifications);

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

        if self.items.get(&new_id).is_some() {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Item with id '{}' already exists",
                new_id
            )));
        }

        let Some(mut resource) = self.items.remove(&old_id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Item with id '{}' not found",
                old_id
            )));
        };

        let data = resource.data().clone();

        resource.rename(notifications, new_id.clone());
        self.items.insert(new_id.clone(), resource);

        let mut new_map = self.data.as_ref().clone();
        new_map.remove(&old_id);
        new_map.insert(new_id, data);
        self.data = Arc::new(new_map);

        Ok(())
    }
}

#[derive(Debug)]
struct ResourceModel {
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

#[derive(Debug)]
struct StyleModel {
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

fn check_id(id: &str) -> Result<(), ProjectManagerActorError> {
    if id.contains(':') {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Id '{}' contains forbidden character ':'",
            id
        )));
    }
    Ok(())
}
