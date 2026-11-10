use std::sync::Arc;

use studio_web_api::{project_manager, utils::PartialField};

use crate::services::project_manager::ui_project::style::StyleOps;

pub trait ControlOps {
    fn new(x: i32, y: i32, r#type: project_manager::UiControlType) -> Self;
    fn set_properties(
        &self,
        properties: project_manager::UiControlProperties,
    ) -> Option<Arc<project_manager::UiControlData>>;
}

impl ControlOps for Arc<project_manager::UiControlData> {
    fn new(x: i32, y: i32, r#type: project_manager::UiControlType) -> Self {
        let (display, text) = match r#type {
            project_manager::UiControlType::Display => (
                Some(Arc::new(project_manager::ControlDisplay {
                    component_id: None,
                    component_state: None,
                    default_resource: None,
                    map: Vec::new(),
                })),
                None,
            ),
            project_manager::UiControlType::Text => (
                None,
                Some(Arc::new(project_manager::UiControlTextData {
                    context: Vec::new(),
                    format: String::from("return '';"),
                })),
            ),
        };

        let control = project_manager::UiControlData {
            x,
            y,
            style: project_manager::Style(Vec::new()),
            height: 50,
            width: 50,
            display,
            text,
            primary_action: None,
            secondary_action: None,
        };

        Arc::new(control)
    }

    fn set_properties(
        &self,
        properties: project_manager::UiControlProperties,
    ) -> Option<Arc<project_manager::UiControlData>> {
        let mut control = self.as_ref().clone();
        let mut changed = false;

        if let PartialField::Value(mut style) = properties.style {
            style.normalize();
            control.style = style;
            changed = true;
        }

        if let PartialField::Value(height) = properties.height {
            control.height = height;
            changed = true;
        }

        if let PartialField::Value(width) = properties.width {
            control.width = width;
            changed = true;
        }

        if let PartialField::Value(x) = properties.x {
            control.x = x;
            changed = true;
        }

        if let PartialField::Value(y) = properties.y {
            control.y = y;
            changed = true;
        }

        if let PartialField::Value(display) = properties.display {
            control.display = display.map(Arc::new);
            changed = true;
        }

        if let PartialField::Value(text) = properties.text {
            control.text = text.map(Arc::new);
            changed = true;
        }

        if let PartialField::Value(primary_action) = properties.primary_action {
            control.primary_action = primary_action.map(Arc::new);
            changed = true;
        }

        if let PartialField::Value(secondary_action) = properties.secondary_action {
            control.secondary_action = secondary_action.map(Arc::new);
            changed = true;
        }

        if changed {
            Some(Arc::new(control))
        } else {
            None
        }
    }
}
