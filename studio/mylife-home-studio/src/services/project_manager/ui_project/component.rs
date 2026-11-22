use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use studio_web_api::project_manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentUsage {
    pub component_id: String,
    pub member_name: String,
    pub path: project_manager::UiElementPath,
}

#[derive(Debug, Clone)]
pub struct ComponentUsageCollector {
    usage: Rc<RefCell<Vec<ComponentUsage>>>,
    path: project_manager::UiElementPath,
}

impl ComponentUsageCollector {
    pub fn new() -> Self {
        Self {
            usage: Rc::new(RefCell::new(Vec::new())),
            path: Vec::new(),
        }
    }

    pub fn at(&self, node_type: impl Into<String>, id: impl Into<String>) -> Self {
        let mut path = self.path.clone();
        path.push(Arc::new(project_manager::UiElementPathNode {
            r#type: node_type.into(),
            id: id.into(),
        }));

        Self {
            path,
            ..self.clone()
        }
    }

    pub fn add_usage(&self, component_id: &str, member_name: &str) {
        let usage = ComponentUsage {
            component_id: component_id.to_string(),
            member_name: member_name.to_string(),
            path: self.path.clone(),
        };
        self.usage.borrow_mut().push(usage);
    }

    pub fn usage(&self) -> Vec<ComponentUsage> {
        self.usage.borrow().clone()
    }
}
