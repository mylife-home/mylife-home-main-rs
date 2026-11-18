use std::{collections::HashMap, sync::Arc};

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::validation::{Validation, ValidationResult},
};

pub trait ArcMapOps<Value: Clone> {
    fn with_inserted(
        &self,
        id: String,
        value: Value,
    ) -> Result<Arc<HashMap<String, Value>>, ProjectManagerActorError>;
    fn with_updated(
        &self,
        id: String,
        value: Value,
    ) -> Result<Arc<HashMap<String, Value>>, ProjectManagerActorError>;
    fn with_set(
        &self,
        id: String,
        value: Value,
    ) -> Result<Arc<HashMap<String, Value>>, ProjectManagerActorError>;
    fn with_cleared(
        &self,
        id: &str,
    ) -> Result<Option<Arc<HashMap<String, Value>>>, ProjectManagerActorError>;
    fn with_renamed(
        &self,
        id: &str,
        new_id: &str,
    ) -> Result<Option<Arc<HashMap<String, Value>>>, ProjectManagerActorError>;

    fn get_required(&self, id: &str) -> Result<&Value, ProjectManagerActorError>;
    fn check_new_id(&self, id: &str) -> Result<(), ProjectManagerActorError>;

    fn with_values_updated(
        &self,
        update: impl FnMut(&String, &Value) -> Option<Value>,
    ) -> Option<Arc<HashMap<String, Value>>>;

    fn deploy<DeployedItem>(
        &self,
        validation: &Validation,
        item_type: &str,
        item_deploy: impl FnMut(&Validation, &String, &Value) -> ValidationResult<DeployedItem>,
    ) -> ValidationResult<Vec<DeployedItem>>;
}

impl<Value: Clone> ArcMapOps<Value> for Arc<HashMap<String, Value>> {
    fn with_inserted(
        &self,
        id: String,
        value: Value,
    ) -> Result<Arc<HashMap<String, Value>>, ProjectManagerActorError> {
        self.check_new_id(&id)?;

        let mut new_map = self.as_ref().clone();
        new_map.insert(id, value);
        Ok(Arc::new(new_map))
    }

    fn with_updated(
        &self,
        id: String,
        value: Value,
    ) -> Result<Arc<HashMap<String, Value>>, ProjectManagerActorError> {
        // ensure update
        self.get_required(&id)?;

        let mut new_map = self.as_ref().clone();
        new_map.insert(id, value);
        Ok(Arc::new(new_map))
    }

    fn with_set(
        &self,
        id: String,
        value: Value,
    ) -> Result<Arc<HashMap<String, Value>>, ProjectManagerActorError> {
        check_id(&id)?;

        let mut new_map = self.as_ref().clone();
        new_map.insert(id, value);
        Ok(Arc::new(new_map))
    }

    fn with_cleared(
        &self,
        id: &str,
    ) -> Result<Option<Arc<HashMap<String, Value>>>, ProjectManagerActorError> {
        let mut new_map = self.as_ref().clone();
        if new_map.remove(id).is_some() {
            Ok(Some(Arc::new(new_map)))
        } else {
            Ok(None)
        }
    }

    fn with_renamed(
        &self,
        id: &str,
        new_id: &str,
    ) -> Result<Option<Arc<HashMap<String, Value>>>, ProjectManagerActorError> {
        if id == new_id {
            return Ok(None);
        }

        self.check_new_id(new_id)?;

        let mut new_map = self.as_ref().clone();
        let value = new_map.remove(id).ok_or_else(|| {
            ProjectManagerActorError::DataProcessingError(format!("Key not found '{}'", id))
        })?;
        new_map.insert(new_id.to_string(), value);
        Ok(Some(Arc::new(new_map)))
    }

    fn get_required(&self, id: &str) -> Result<&Value, ProjectManagerActorError> {
        self.get(id).ok_or_else(|| {
            ProjectManagerActorError::DataProcessingError(format!("Key not found '{}'", id))
        })
    }

    fn check_new_id(&self, id: &str) -> Result<(), ProjectManagerActorError> {
        check_id(id)?;

        if self.contains_key(id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Id '{}' already exists",
                id
            )));
        }

        Ok(())
    }

    fn with_values_updated(
        &self,
        mut update: impl FnMut(&String, &Value) -> Option<Value>,
    ) -> Option<Arc<HashMap<String, Value>>> {
        let mut new_map = self.as_ref().clone();
        let mut changed = false;

        for (key, value) in new_map.iter_mut() {
            if let Some(updated_value) = update(key, &value) {
                *value = updated_value;
                changed = true;
            }
        }

        if changed {
            Some(Arc::new(new_map))
        } else {
            None
        }
    }

    fn deploy<DeployedItem>(
        &self,
        validation: &Validation,
        item_type: &str,
        mut item_deploy: impl FnMut(&Validation, &String, &Value) -> ValidationResult<DeployedItem>,
    ) -> ValidationResult<Vec<DeployedItem>> {
        // force to collect all items (and don't stop at first error)
        let result: Vec<_> = self
            .iter()
            .map(|(id, value)| item_deploy(&validation.at(item_type, id), id, value))
            .collect();
        result.into_iter().collect()
    }
}

pub trait VecOps<Value> {
    fn deploy<DeployedItem>(
        &self,
        validation: &Validation,
        item_type: &str,
        item_deploy: impl FnMut(&Validation, &Value) -> ValidationResult<DeployedItem>,
    ) -> ValidationResult<Vec<DeployedItem>>;
}

impl<Value> VecOps<Value> for Vec<Value> {
    fn deploy<DeployedItem>(
        &self,
        validation: &Validation,
        item_type: &str,
        mut item_deploy: impl FnMut(&Validation, &Value) -> ValidationResult<DeployedItem>,
    ) -> ValidationResult<Vec<DeployedItem>> {
        // force to collect all items (and don't stop at first error)
        let result: Vec<_> = self
            .iter()
            .enumerate()
            .map(|(index, value)| item_deploy(&validation.at(item_type, &index.to_string()), value))
            .collect();
        result.into_iter().collect()
    }
}

pub fn check_id(id: &str) -> Result<(), ProjectManagerActorError> {
    if id.trim().is_empty() {
        return Err(ProjectManagerActorError::DataProcessingError(String::from(
            "Id is empty",
        )));
    }

    if id.contains(':') {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Id '{}' contains forbidden character ':'",
            id
        )));
    }
    Ok(())
}
