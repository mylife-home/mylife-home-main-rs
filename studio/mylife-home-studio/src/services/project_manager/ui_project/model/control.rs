use std::sync::Arc;

use studio_web_api::{project_manager, utils::PartialField};

use crate::services::project_manager::ui_project::model::view::ViewItem;

#[derive(Debug)]
pub struct ControlModel {
    id: String,
    data: Arc<project_manager::UiControlData>,
}

impl ViewItem for ControlModel {
    type UiData = project_manager::UiControlData;

    fn new(id: &String, data: &Arc<project_manager::UiControlData>) -> Self {
        Self {
            id: id.clone(),
            data: data.clone(),
        }
    }

    fn data(&self) -> &Arc<project_manager::UiControlData> {
        &self.data
    }

    fn rename(&mut self, new_id: String) {
        self.id = new_id;
    }
}

impl ControlModel {

    pub fn create(id: String, r#type: project_manager::UiControlType, x: i32, y: i32) -> Self {
        let mut data = project_manager::UiControlData {
            x,
            y,
            style: project_manager::Style(Vec::new()),
            height: 50,
            width: 50,
            display: Arc::new(None),
            text: Arc::new(None),
            primary_action: Arc::new(None),
            secondary_action: Arc::new(None),
        };

        match r#type {
            project_manager::UiControlType::Display => {
                data.display = Arc::new(Some(project_manager::ControlDisplay {
                    component_id: None,
                    component_state: None,
                    default_resource: None,
                    map: Vec::new(),
                }));
            }

            project_manager::UiControlType::Text => {
                data.text = Arc::new(Some(project_manager::UiControlTextData {
                    context: Vec::new(),
                    format: String::from("return '';"),
                }));
            }
        }

        Self {
            id: id.clone(),
            data: Arc::new(data),
        }
    }

    pub fn update(&mut self, properties: project_manager::UiControlDataPartial) {
        let mut new_data = self.data.as_ref().clone();

        if let PartialField::Value(style) = properties.style {
            let mut style = style.0;
            style.sort();
            new_data.style = project_manager::Style(style);
        }

        if let PartialField::Value(height) = properties.height {
            new_data.height = height;
        }

        if let PartialField::Value(width) = properties.width {
            new_data.width = width;
        }

        if let PartialField::Value(x) = properties.x {
            new_data.x = x;
        }

        if let PartialField::Value(y) = properties.y {
            new_data.y = y;
        }

        if let PartialField::Value(display) = properties.display {
            new_data.display = Arc::new(display);
        }

        if let PartialField::Value(text) = properties.text {
            new_data.text = Arc::new(text);
        }

        if let PartialField::Value(primary_action) = properties.primary_action {
            new_data.primary_action = Arc::new(primary_action);
        }

        if let PartialField::Value(secondary_action) = properties.secondary_action {
            new_data.secondary_action = Arc::new(secondary_action);
        }

        self.data = Arc::new(new_data);
    }
}
