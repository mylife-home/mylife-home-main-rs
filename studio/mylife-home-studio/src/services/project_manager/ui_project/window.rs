use std::collections::HashMap;
use std::sync::Arc;
use studio_web_api::utils::PartialField;
use studio_web_api::{project_manager, ui_model};

use crate::services::project_manager::ui_project::control::ControlOps;
use crate::services::project_manager::ui_project::resource::OptionResourceOps;
use crate::services::project_manager::ui_project::style::StyleOps;
use crate::services::project_manager::ui_project::template_instance::TemplateInstanceOps;
use crate::services::project_manager::ui_project::utils::ArcMapOps;
use crate::services::project_manager::ui_project::validation::{
    InvalidForDeployment, Validation, ValidationResult,
};
use crate::services::project_manager::ui_project::view::{DeployPosition, View};

pub trait WindowOps {
    fn deploy(&self, id: &str, validation: &Validation) -> ValidationResult<ui_model::Window>;

    fn new() -> Arc<project_manager::UiWindowData>;

    fn set_properties(
        &self,
        properties: project_manager::UiWindowProperties,
    ) -> Option<Arc<project_manager::UiWindowData>>;

    fn on_change_resource(
        &self,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiWindowData>>;

    fn on_change_style(
        &self,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiWindowData>>;

    fn on_change_window(
        &self,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiWindowData>>;

    fn on_change_template(
        &self,
        id: &str,
        new_id: Option<&str>,
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

    fn on_change_resource(
        &self,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiWindowData>> {
        let new_controls = self
            .controls
            .with_values_updated(|_, control| control.on_change_resource(id, new_id));

        let new_background_resource = self.background_resource.on_change_resource(id, new_id);

        if new_controls.is_none() && new_background_resource.is_none() {
            return None;
        }

        let mut window = self.as_ref().clone();
        if let Some(new_controls) = new_controls {
            window.controls = new_controls;
        }
        if let Some(background_resource) = new_background_resource {
            window.background_resource = background_resource;
        }

        Some(Arc::new(window))
    }

    fn on_change_style(
        &self,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiWindowData>> {
        let new_controls = self
            .controls
            .with_values_updated(|_, control| control.on_change_style(id, new_id));

        let new_style = self.style.on_change(id, new_id);

        if new_controls.is_none() && new_style.is_none() {
            return None;
        }

        let mut window = self.as_ref().clone();
        if let Some(new_controls) = new_controls {
            window.controls = new_controls;
        }
        if let Some(new_style) = new_style {
            window.style = new_style;
        }

        Some(Arc::new(window))
    }

    fn on_change_window(
        &self,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiWindowData>> {
        let Some(new_controls) = self
            .controls
            .with_values_updated(|_, control| control.on_change_window(id, new_id))
        else {
            return None;
        };

        let mut window = self.as_ref().clone();
        window.controls = new_controls;
        Some(Arc::new(window))
    }

    fn on_change_template(
        &self,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiWindowData>> {
        let new_templates = if let Some(new_id) = new_id {
            // template rename: update all instances that use this template
            self.templates
                .with_values_updated(|_, template| template.on_rename_template(id, new_id))
        } else {
            // template remove: remove all instances that were using it
            let mut new_templates = self.templates.as_ref().clone();
            new_templates.retain(|_, template| template.template_id != id);
            if new_templates.len() < self.templates.len() {
                Some(Arc::new(new_templates))
            } else {
                None
            }
        };

        if new_templates.is_none() {
            return None;
        }

        let mut window = self.as_ref().clone();
        window.templates = new_templates.unwrap();
        Some(Arc::new(window))
    }
}
