use std::sync::Arc;

use crate::components::types::Value;

#[derive(Debug, Clone)]
pub struct Action {
    component_id: Arc<String>,
    name: Arc<String>,
    value: Arc<Value>,
}

impl Action {
    pub fn new(component_id: String, name: String, value: Value) -> Self {
        Self {
            component_id: Arc::new(component_id),
            name: Arc::new(name),
            value: Arc::new(value),
        }
    }

    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &Value {
        &self.value
    }
}

#[derive(Debug, Clone)]
pub struct State {
    component_id: Arc<String>,
    name: Arc<String>,
    value: Arc<Value>,
}

impl State {
    pub fn new(component_id: String, name: String, value: Value) -> Self {
        Self {
            component_id: Arc::new(component_id),
            name: Arc::new(name),
            value: Arc::new(value),
        }
    }

    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &Value {
        &self.value
    }
}
