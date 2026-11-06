use std::collections::HashMap;

use studio_web_api::project_manager;

use std::sync::Arc;

use crate::services::project_manager::{ProjectManagerActorError, ui_project::model::{
    collection::check_id, control::ControlModel, template_instance::TemplateInstanceModel,
}};

#[derive(Debug)]
pub struct ViewModel {
    id: String,
    controls: ViewItemCollection<ControlModel>,
    templates: ViewItemCollection<TemplateInstanceModel>,
}

impl ViewModel {
    pub fn new(
        id: String,
        controls: Arc<HashMap<String, Arc<project_manager::UiControlData>>>,
        templates: Arc<HashMap<String, Arc<project_manager::UiTemplateInstanceData>>>,
    ) -> Self {
        Self {
            id,
            controls: ViewItemCollection::new(controls),
            templates: ViewItemCollection::new(templates),
        }
    }

    pub fn controls(&self) -> &Arc<HashMap<String, Arc<project_manager::UiControlData>>> {
        self.controls.data()
    }

    pub fn templates(&self) -> &Arc<HashMap<String, Arc<project_manager::UiTemplateInstanceData>>> {
        self.templates.data()
    }

    pub fn new_control(
        &mut self,
        id: String,
        r#type: project_manager::UiControlType,
        x: i32,
        y: i32,
    ) -> Result<&ControlModel, ProjectManagerActorError> {
        check_id(&id)?;

        let control = ControlModel::create(id.clone(), r#type, x, y);
        Ok(self.controls.set(&id, control))
    }

    pub fn clone_control(
        &mut self,
        source_id: String,
        new_id: String,
    ) -> Result<&ControlModel, ProjectManagerActorError> {
        check_id(&new_id)?;
        
        let source = self.controls.get(&source_id).ok_or(ProjectManagerActorError::DataProcessingError(format!(
            "Control with id '{}' not found",
            source_id
        )))?;

        let mut new_control = source.data().clone();

        if source.owner() == self {
            new_control.x += 10;
            new_control.y += 10;
        } else {
            new_control.x = 10;
            new_control.y = 10;
        }

        let model = ControlModel::new(&new_id, &Arc::new(new_control));
        Ok(self.controls.set(&new_id, model))
    }

    pub fn clear_control(
        &mut self,
        id: &String,
    ) {
        self.controls.clear(id)
    }

    pub fn rename_control(
        &mut self,
        id: &String,
        new_id: &String,
    ) -> Result<(), ProjectManagerActorError> {
        self.controls.rename(id, new_id)
    }

    pub fn get_control(&self, id: &String) -> Option<&ControlModel> {
        self.controls.get(id)
    }

    pub fn get_control_mut(&mut self, id: &String) -> Option<&mut ControlModel> {
        self.controls.get_mut(id)
    }

  newTemplateInstance(templateInstanceId: string, templateId: string, x: number, y: number) {
    const template = this.project.getTemplate(templateId);
    const newTemplateInstance = clone(TEMPLATE_INSTANCE_TEMPLATE) as UiTemplateInstanceData;
    newTemplateInstance.x = x;
    newTemplateInstance.y = y;

    const model = this.templates.set(templateInstanceId, newTemplateInstance);
    model.setTemplate(template);
  }

    pub fn clone_template_instance(
        &mut self,
        source_id: String,
        new_id: String,
    ) -> Result<&TemplateInstanceModel, ProjectManagerActorError> {
        check_id(&new_id)?;
        
        let source = self.templates.get(&source_id).ok_or(ProjectManagerActorError::DataProcessingError(format!(
            "Template instance with id '{}' not found",
            source_id
        )))?;

        let mut new_data = source.data().clone();
        new_data.x += 10;
        new_data.y += 10;

        let model = TemplateInstanceModel::new(&new_id, &Arc::new(new_data));
        Ok(self.templates.set(&new_id, model))
    }

    pub fn clear_template_instance(
        &mut self,
        id: &String,
    ) {
        self.templates.clear(id)
    }

    pub fn rename_template_instance(
        &mut self,
        id: &String,
        new_id: &String,
    ) -> Result<(), ProjectManagerActorError> {
        self.templates.rename(id, new_id)
    }

    pub fn get_template_instance(&self, id: &String) -> Option<&TemplateInstanceModel> {
        self.templates.get(id)
    }

    pub fn get_template_instance_mut(&mut self, id: &String) -> Option<&mut TemplateInstanceModel> {
        self.templates.get_mut(id)
    }
}

pub trait ViewItem {
    type UiData;

    fn new(id: &String, data: &Arc<Self::UiData>) -> Self;
    fn data(&self) -> &Arc<Self::UiData>;
    fn rename(&mut self, new_id: String);
}

#[derive(Debug)]
struct ViewItemCollection<Model: ViewItem> {
    data: Arc<HashMap<String, Arc<Model::UiData>>>,
    items: HashMap<String, Model>,
}

impl<Model: ViewItem> ViewItemCollection<Model> {
    pub fn new(data: Arc<HashMap<String, Arc<Model::UiData>>>) -> Self {
        let items = data
            .iter()
            .map(|(key, item_data)| (key.clone(), Model::new(key, item_data)))
            .collect();

        Self { data, items }
    }

    pub fn data(&self) -> &Arc<HashMap<String, Arc<Model::UiData>>> {
        &self.data
    }

    pub fn set(&mut self, id: &String, model: Model) -> &Model {
        let new_data = model.data().clone();
        self.items.insert(id.clone(), model);

        // Rebuild the map
        let mut new_map = self.data.as_ref().clone();
        new_map.insert(id.clone(), new_data);

        self.data = Arc::new(new_map);

        self.items.get(id).expect("data inconsistency")
    }

    pub fn clear(&mut self, id: &String) {
        if self.items.remove(id).is_none() {
            return;
        }

        // Rebuild the map
        let mut new_map = self.data.as_ref().clone();
        new_map.remove(id);
        self.data = Arc::new(new_map);
    }

    pub fn rename(&mut self, id: &String, new_id: &String) -> Result<(), ProjectManagerActorError> {
        if self.items.get(new_id).is_some() {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Item with id '{}' already exists",
                new_id
            )));
        }

        let Some(mut resource) = self.items.remove(id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Item with id '{}' not found",
                id
            )));
        };

        let data = resource.data().clone();

        resource.rename(new_id.clone());
        self.items.insert(new_id.clone(), resource);

        let mut new_map = self.data.as_ref().clone();
        new_map.remove(id);
        new_map.insert(new_id.clone(), data);
        self.data = Arc::new(new_map);

        Ok(())
    }

    pub fn get(&self, id: &String) -> Option<&Model> {
        self.items.get(id)
    }

    pub fn get_mut(&mut self, id: &String) -> Option<&mut Model> {
        self.items.get_mut(id)
    }
}
