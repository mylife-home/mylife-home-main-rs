use std::{sync::Arc, todo};
use studio_web_api::{project_manager, utils::PartialField};

use crate::services::project_manager::ui_project::model::{template::TemplateModel, view::ViewItem};

#[derive(Debug)]
pub struct TemplateInstanceModel {
    id: String,
    data: Arc<project_manager::UiTemplateInstanceData>,
}

impl ViewItem for TemplateInstanceModel {
    type UiData = project_manager::UiTemplateInstanceData;

    fn new(id: &String, data: &Arc<project_manager::UiTemplateInstanceData>) -> Self {
        Self {
            id: id.clone(),
            data: data.clone(),
        }
    }

    fn data(&self) -> &Arc<project_manager::UiTemplateInstanceData> {
        &self.data
    }

    fn rename(&mut self, new_id: String) {
        self.id = new_id;
    }
}

impl TemplateInstanceModel {
    pub fn r#move(&mut self, x: &PartialField<i32>, y: &PartialField<i32>) {
        let mut new_data = self.data.as_ref().clone();

        if let PartialField::Value(x) = x {
            new_data.x = *x;
        }

        if let PartialField::Value(y) = y {
            new_data.y = *y;
        }

        self.data = Arc::new(new_data);
    }

    pub fn set_template(&mut self, template: &Arc<TemplateModel>) {
        let mut new_data = self.data.as_ref().clone();

        new_data.template_id = template.id().clone();

        // reset bindings
        todo!();
        //new_data.bindings = template.prepareBindings();

        self.data = Arc::new(new_data);
    }

    pub fn set_binding(&mut self, export_id: &str, component_id: &str, member_name: &str) {
        todo!()
    }

    pub fn init_binding(&mut self, export_id: &str) {
        todo!()
    }

    pub fn clear_binding(&mut self, export_id: &str) {
        todo!()
    }
}
