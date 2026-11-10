use std::{collections::HashMap, sync::Arc};

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::old::{
        component::ComponentOperation, operation::ProjectOperation, template::TemplateOperation,
        view::ViewOperation,
    },
};
pub trait TemplateInstanceOperation {
    fn template_instance_new(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        template_id: String,
        x: i32,
        y: i32,
    ) -> Result<(), ProjectManagerActorError>;
    fn template_instance_clear(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
    ) -> Result<(), ProjectManagerActorError>;
    fn template_instance_rename(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError>;
    fn template_instance_clone(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError>;
    fn template_instance_move(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        x: i32,
        y: i32,
    ) -> Result<(), ProjectManagerActorError>;
    fn template_instance_set_template(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        template_id: String,
    ) -> Result<(), ProjectManagerActorError>;
    fn template_instance_set_bindings(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        bindings: HashMap<String, project_manager::UiTemplateInstanceBinding>,
    ) -> Result<(), ProjectManagerActorError>;
}

impl TemplateInstanceOperation for ProjectOperation {
    fn template_instance_new(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        template_id: String,
        x: i32,
        y: i32,
    ) -> Result<(), ProjectManagerActorError> {
        self.check_id(&id)?;

        let bindings = self.template_prepare_bindings(&template_id)?;

        self.view_update_template_instances(view_type, view_id, |_, templates| {
            if templates.contains_key(&id) {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Template instance with id '{}' already exists",
                    id
                )));
            }

            let template = project_manager::UiTemplateInstanceData {
                template_id,
                x,
                y,
                bindings,
            };

            templates.insert(id, Arc::new(template));

            Ok(true)
        })?;

        Ok(())
    }

    fn template_instance_clear(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.view_update_template_instances(view_type, view_id, |_, templates| {
            Ok(templates.remove(&id).is_some())
        })?;

        Ok(())
    }

    fn template_instance_rename(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.check_id(&new_id)?;

        if id == new_id {
            return Ok(());
        }

        self.view_update_template_instances(view_type, view_id, |_, templates| {
            if templates.contains_key(&new_id) {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Template instance with id '{}' already exists",
                    new_id
                )));
            }

            let Some(template) = templates.remove(&id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Template instance with id '{}' does not exist",
                    id
                )));
            };

            templates.insert(new_id, template);

            Ok(true)
        })?;

        Ok(())
    }

    fn template_instance_clone(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.check_id(&new_id)?;

        self.view_update_template_instances(view_type, view_id, |_, templates| {
            if templates.contains_key(&new_id) {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Template instance with id '{}' already exists",
                    new_id
                )));
            }

            let Some(source) = templates.get(&id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Template instance with id '{}' does not exist",
                    id
                )));
            };

            let mut new_template_instance = source.as_ref().clone();
            new_template_instance.x += 10;
            new_template_instance.y += 10;

            templates.insert(new_id, Arc::new(new_template_instance));

            Ok(true)
        })
    }

    fn template_instance_move(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        x: i32,
        y: i32,
    ) -> Result<(), ProjectManagerActorError> {
        self.view_update_template_instances(view_type, view_id, |_, templates| {
            let Some(template) = templates.get(&id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Template instance with id '{}' does not exist",
                    id
                )));
            };

            let mut template = template.as_ref().clone();
            template.x = x;
            template.y = y;

            templates.insert(id, Arc::new(template));

            Ok(true)
        })?;

        Ok(())
    }

    fn template_instance_set_template(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        template_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        let bindings = self.template_prepare_bindings(&template_id)?;

        self.view_update_template_instances(view_type, view_id, |_, templates| {
            let Some(template_instance) = templates.get(&id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Template instance with id '{}' does not exist",
                    id
                )));
            };

            let mut template_instance = template_instance.as_ref().clone();
            template_instance.template_id = template_id;
            // reset bindings
            template_instance.bindings = bindings;

            templates.insert(id, Arc::new(template_instance));

            Ok(true)
        })?;

        Ok(())
    }

    fn template_instance_set_bindings(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        bindings: HashMap<String, project_manager::UiTemplateInstanceBinding>,
    ) -> Result<(), ProjectManagerActorError> {
        self.view_update_template_instances(view_type, view_id.clone(), |self_, templates| {
            let Some(template_instance) = templates.get(&id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Template instance with id '{}' does not exist",
                    id
                )));
            };

            let mut template_instance = template_instance.as_ref().clone();
            let mut new_bindings = template_instance.bindings.as_ref().clone();

            let template = self_.template_get(&template_instance.template_id)?.clone();

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
                        export_id, id
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
                        export_id, template_instance.template_id
                    )));
                };

                let value_type = if let Some(component_id) = &component_id {
                    self_.component_find_member_value_type(
                        component_id,
                        &member_name,
                        export_data.member_type,
                    )
                } else if view_type == project_manager::ViewType::Template {
                    self_.template_get_export_value_type(
                        &view_id,
                        &member_name,
                        export_data.member_type,
                    )?
                } else {
                    None
                };

                if value_type != Some(export_data.value_type.clone()) {
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

            template_instance.bindings = Arc::new(new_bindings);
            templates.insert(id, Arc::new(template_instance));

            Ok(true)
        })?;

        Ok(())
    }
}
