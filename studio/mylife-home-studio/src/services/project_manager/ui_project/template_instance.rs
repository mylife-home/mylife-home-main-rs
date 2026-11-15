use std::sync::Arc;

use std::collections::HashMap;
use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{project::ProjectOps, template::TemplateOps, utils::ArcMapOps},
};

pub trait TemplateInstanceOps {
    fn new(
        template_id: String,
        template: &Arc<project_manager::UiTemplateData>,
        x: i32,
        y: i32,
    ) -> Result<Arc<project_manager::UiTemplateInstanceData>, ProjectManagerActorError>;

    fn clone_new(&self) -> Arc<project_manager::UiTemplateInstanceData>;

    fn r#move(&self, x: i32, y: i32) -> Arc<project_manager::UiTemplateInstanceData>;

    fn set_template(
        &self,
        template_id: String,
        template: &Arc<project_manager::UiTemplateData>,
    ) -> Arc<project_manager::UiTemplateInstanceData>;

    fn set_bindings(
        &self,
        project: &Arc<project_manager::UiProject>,
        owner_template: Option<&Arc<project_manager::UiTemplateData>>,
        bindings: HashMap<String, project_manager::UiTemplateInstanceBinding>,
    ) -> Result<Arc<project_manager::UiTemplateInstanceData>, ProjectManagerActorError>;
}

impl TemplateInstanceOps for Arc<project_manager::UiTemplateInstanceData> {
    fn new(
        template_id: String,
        template: &Arc<project_manager::UiTemplateData>,
        x: i32,
        y: i32,
    ) -> Result<Arc<project_manager::UiTemplateInstanceData>, ProjectManagerActorError> {
        let bindings = template.prepare_bindings();

        Ok(Arc::new(project_manager::UiTemplateInstanceData {
            template_id,
            x,
            y,
            bindings,
        }))
    }

    fn clone_new(&self) -> Arc<project_manager::UiTemplateInstanceData> {
        let mut template = self.as_ref().clone();
        template.x += 10;
        template.y += 10;
        Arc::new(template)
    }

    fn r#move(&self, x: i32, y: i32) -> Arc<project_manager::UiTemplateInstanceData> {
        Arc::new(project_manager::UiTemplateInstanceData {
            x,
            y,
            ..self.as_ref().clone()
        })
    }

    fn set_template(
        &self,
        template_id: String,
        template: &Arc<project_manager::UiTemplateData>,
    ) -> Arc<project_manager::UiTemplateInstanceData> {
        let bindings = template.prepare_bindings();

        Arc::new(project_manager::UiTemplateInstanceData {
            template_id,
            x: self.as_ref().x,
            y: self.as_ref().y,
            bindings,
        })
    }

    fn set_bindings(
        &self,
        project: &Arc<project_manager::UiProject>,
        owner_template: Option<&Arc<project_manager::UiTemplateData>>,
        bindings: HashMap<String, project_manager::UiTemplateInstanceBinding>,
    ) -> Result<Arc<project_manager::UiTemplateInstanceData>, ProjectManagerActorError> {
        let template = project.templates.get_required(&self.template_id)?;
        let mut new_bindings = self.bindings.as_ref().clone();

        for (
            export_id,
            project_manager::UiTemplateInstanceBinding {
                component_id,
                member_name,
            },
        ) in bindings
        {
            let Some(binding) = new_bindings.get_mut(&export_id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Binding from export '{}' not found on template '{}'",
                    export_id, self.template_id
                )));
            };

            let mut binding = binding.as_ref().clone();

            let Some(member_name) = member_name else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Member name not provided for export '{}'",
                    export_id
                )));
            };

            let Some(export_data) = template.exports.get(&export_id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Export '{}' does not exist on template '{}'",
                    export_id, self.template_id
                )));
            };

            let value_type = if let Some(component_id) = &component_id {
                project.find_component_member_value_type(
                    component_id,
                    &member_name,
                    export_data.member_type,
                )?
            } else if let Some(owner_template) = owner_template {
                owner_template.get_export_value_type(&member_name, export_data.member_type)?
            } else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Cannot determine value type for export '{}'",
                    export_id
                )));
            };

            if *value_type != export_data.value_type {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Binding mismatch: cannot bind '{}.{}' on export '{}'",
                    component_id.as_deref().unwrap_or("<template>"),
                    member_name,
                    export_id
                )));
            }

            binding.component_id = component_id;
            binding.member_name = Some(member_name);
            new_bindings.insert(export_id, Arc::new(binding));
        }

        Ok(Arc::new(project_manager::UiTemplateInstanceData {
            bindings: Arc::new(new_bindings),
            ..self.as_ref().clone()
        }))
    }
}
