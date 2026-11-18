use std::collections::HashMap;
use std::sync::Arc;
use studio_web_api::{project_manager, ui_model, utils::PartialField};

use crate::services::project_manager::ui_project::{
    style::StyleOps,
    validation::{InvalidForDeployment, Validation, ValidationResult},
    view::{DeployPosition, View},
};

pub trait WindowOps {
    fn deploy(&self, id: &str, validation: &Validation) -> ValidationResult<ui_model::Window>;

    fn new() -> Arc<project_manager::UiWindowData>;

    fn set_properties(
        &self,
        properties: project_manager::UiWindowProperties,
    ) -> Option<Arc<project_manager::UiWindowData>>;
}

impl WindowOps for Arc<project_manager::UiWindowData> {
    fn deploy(&self, id: &str, validation: &Validation) -> ValidationResult<ui_model::Window> {
        let background = validation.validate_optional_resource(&self.background_resource);

        let controls = {
            let view = View::window(self.clone());
            view.deploy_controls(validation, &DeployPosition::default())
        };

        let (Ok(background_resource), Ok(controls)) = (background, controls) else {
            return Err(InvalidForDeployment);
        };

        Ok(ui_model::Window {
            id: id.to_owned(),
            style: self.style.clone(),
            height: self.height,
            width: self.width,
            background_resource,
            controls,
        })
    }

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

        changed.then(|| Arc::new(window))
    }
}
