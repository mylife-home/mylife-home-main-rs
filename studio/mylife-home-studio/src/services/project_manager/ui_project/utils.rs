use std::{collections::HashMap, sync::Arc};

use crate::services::project_manager::ProjectManagerActorError;

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
    ) -> Result<Arc<HashMap<String, Value>>, ProjectManagerActorError>;
    fn with_renamed(
        &self,
        id: &str,
        new_id: &str,
    ) -> Result<Arc<HashMap<String, Value>>, ProjectManagerActorError>;

    fn get_required(&self, id: &String) -> Result<&Value, ProjectManagerActorError>;
    fn check_new_id(&self, id: &str) -> Result<(), ProjectManagerActorError>;
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
        let mut new_map = self.as_ref().clone();
        new_map.insert(id, value);
        Ok(Arc::new(new_map))
    }

    fn with_cleared(
        &self,
        id: &str,
    ) -> Result<Arc<HashMap<String, Value>>, ProjectManagerActorError> {
        let mut new_map = self.as_ref().clone();
        new_map.remove(id);
        Ok(Arc::new(new_map))
    }

    fn with_renamed(
        &self,
        id: &str,
        new_id: &str,
    ) -> Result<Arc<HashMap<String, Value>>, ProjectManagerActorError> {
        self.check_new_id(new_id)?;

        let mut new_map = self.as_ref().clone();
        let value = new_map.remove(id).ok_or_else(|| {
            ProjectManagerActorError::DataProcessingError(format!("Key not found '{}'", id))
        })?;
        new_map.insert(new_id.to_string(), value);
        Ok(Arc::new(new_map))
    }

    fn get_required(&self, id: &String) -> Result<&Value, ProjectManagerActorError> {
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
