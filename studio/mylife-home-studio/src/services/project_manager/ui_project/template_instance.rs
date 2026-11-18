use std::sync::Arc;

use std::collections::HashMap;
use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{
        project::ProjectOps,
        template::TemplateOps,
        utils::ArcMapOps,
        validation::{InvalidForDeployment, Validation, ValidationResult},
    },
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

    fn on_rename_template(
        &self,
        id: &str,
        new_id: &str,
    ) -> Option<Arc<project_manager::UiTemplateInstanceData>>;

    fn on_clear_template_export(
        &self,
        export_id: &str,
    ) -> Option<Arc<project_manager::UiTemplateInstanceData>>;

    fn deploy(
        &self,
        validation: &Validation,
    ) -> ValidationResult<(
        Arc<project_manager::UiTemplateData>,
        HashMap<String, project_manager::UiTemplateInstanceBinding>,
    )>;
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

    fn on_rename_template(
        &self,
        id: &str,
        new_id: &str,
    ) -> Option<Arc<project_manager::UiTemplateInstanceData>> {
        if self.template_id == id {
            Some(Arc::new(project_manager::UiTemplateInstanceData {
                template_id: new_id.to_string(),
                ..self.as_ref().clone()
            }))
        } else {
            None
        }
    }

    fn on_clear_template_export(
        &self,
        export_id: &str,
    ) -> Option<Arc<project_manager::UiTemplateInstanceData>> {
        let mut bindings = self.bindings.as_ref().clone();
        let mut changed = false;

        for (binding_id, binding) in self.bindings.iter() {
            if binding.component_id.is_none() && binding.member_name.as_deref() == Some(export_id) {
                bindings.insert(
                    binding_id.clone(),
                    Arc::new(project_manager::UiTemplateInstanceBinding {
                        member_name: None,
                        ..binding.as_ref().clone()
                    }),
                );
                changed = true;
            }
        }

        changed.then(|| {
            Arc::new(project_manager::UiTemplateInstanceData {
                bindings: Arc::new(bindings),
                ..self.as_ref().clone()
            })
        })
    }

    fn deploy(
        &self,
        validation: &Validation,
    ) -> ValidationResult<(
        Arc<project_manager::UiTemplateData>,
        HashMap<String, project_manager::UiTemplateInstanceBinding>,
    )> {
        let Some(template) = validation
            .project()
            .templates
            .get(&self.template_id)
            .cloned()
        else {
            validation.add_error(format!("Le template '{}' n'existe pas.", self.template_id));
            return Err(InvalidForDeployment);
        };

        let mut has_errors = false;
        let mut bindings = HashMap::new();
        for binding_id in self.bindings.keys() {
            if !template.exports.contains_key(binding_id) {
                let validation = validation.at("template-binding", binding_id);
                validation.add_error(format!("Binding sur export '{}' inexistant", binding_id));
                has_errors = true;
            }
        }

        for (export_id, export) in template.exports.iter() {
            let Some(binding) = self.bindings.get(export_id) else {
                let validation = validation.at("template-binding", export_id);
                validation.add_error(format!("Binding inexistant sur export '{}'", export_id));
                has_errors = true;
                continue;
            };

            let mut binding_validation = validation.at("template-binding", export_id);
            let result = binding_validation.validate_component_member(
                &binding.component_id,
                &binding.member_name,
                export.member_type,
                Some(&export.value_type),
                false,
            );
            has_errors |= result.is_err();

            if result.is_ok() {
                let resolved = if binding.component_id.is_some() {
                    binding.as_ref().clone()
                } else {
                    binding
                        .member_name
                        .as_ref()
                        .and_then(|member_name| validation.bindings().get(member_name))
                        .cloned()
                        .unwrap_or_else(|| binding.as_ref().clone())
                };
                bindings.insert(export_id.clone(), resolved);
            }
        }

        if has_errors {
            Err(InvalidForDeployment)
        } else {
            Ok((template, bindings))
        }
    }
}
