use std::collections::HashMap;
use std::sync::Arc;
use studio_web_api::project_manager;
use studio_web_api::utils::PartialField;

use crate::services::project_manager::ui_project::style::StyleOps;

pub trait WindowOps {
    fn new() -> Arc<project_manager::UiWindowData>;
    fn set_properties(
        &self,
        properties: project_manager::UiWindowProperties,
    ) -> Option<Arc<project_manager::UiWindowData>>;
}

impl WindowOps for Arc<project_manager::UiWindowData> {
    fn new() -> Arc<project_manager::UiWindowData> {
        Arc::new(project_manager::UiWindowData {
            style: project_manager::Style(vec![]),
            height: 500,
            width: 500,
            background_resource: None,
            controls: Arc::new(HashMap::new()),
            templates: Arc::new(HashMap::new()),
        })
    }

    fn set_properties(
        &self,
        properties: project_manager::UiWindowProperties,
    ) -> Option<Arc<project_manager::UiWindowData>> {
        let mut window = self.as_ref().clone();
        let mut changed = false;

        if let PartialField::Value(height) = properties.height {
            window.height = height;
            changed = true;
        }

        if let PartialField::Value(width) = properties.width {
            window.width = width;
            changed = true;
        }

        if let PartialField::Value(mut style) = properties.style {
            style.normalize();
            window.style = style;
            changed = true;
        }

        if let PartialField::Value(background_resource) = properties.background_resource {
            window.background_resource = background_resource;
            changed = true;
        }

        if changed {
            Some(Arc::new(window))
        } else {
            None
        }
    }
}
