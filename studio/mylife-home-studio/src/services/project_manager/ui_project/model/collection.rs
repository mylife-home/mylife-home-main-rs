use std::{collections::HashMap, fmt, sync::Arc};

use studio_web_api::project_manager;

use crate::services::project_manager::ProjectManagerActorError;

pub trait CollectionItem: fmt::Debug {
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
pub struct CollectionModel<Item: CollectionItem> {
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

fn check_id(id: &str) -> Result<(), ProjectManagerActorError> {
    if id.contains(':') {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Id '{}' contains forbidden character ':'",
            id
        )));
    }
    Ok(())
}
