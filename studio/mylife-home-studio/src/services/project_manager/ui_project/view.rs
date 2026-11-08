use std::{collections::HashMap, sync::Arc};

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{
        operation::ProjectOperation, template::TemplateOperation, window::WindowOperation,
    },
};
pub trait ViewOperation {
    fn view_update_controls(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        mutator: impl FnOnce(
            &ProjectOperation,
            &mut HashMap<String, Arc<project_manager::UiControlData>>,
        ) -> Result<bool, ProjectManagerActorError>,
    ) -> Result<(), ProjectManagerActorError>;
    fn view_get_control(
        &self,
        view_type: project_manager::ViewType,
        view_id: &str,
        id: &str,
    ) -> Result<&Arc<project_manager::UiControlData>, ProjectManagerActorError>;

    fn view_update_template_instances(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        mutator: impl FnOnce(
            &ProjectOperation,
            &mut HashMap<String, Arc<project_manager::UiTemplateInstanceData>>,
        ) -> Result<bool, ProjectManagerActorError>,
    ) -> Result<(), ProjectManagerActorError>;
    fn view_get_template_instance(
        &self,
        view_type: project_manager::ViewType,
        view_id: &str,
        id: &str,
    ) -> Result<&Arc<project_manager::UiTemplateInstanceData>, ProjectManagerActorError>;
}

impl ViewOperation for ProjectOperation {
    fn view_update_controls(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        mutator: impl FnOnce(
            &ProjectOperation,
            &mut HashMap<String, Arc<project_manager::UiControlData>>,
        ) -> Result<bool, ProjectManagerActorError>,
    ) -> Result<(), ProjectManagerActorError> {
        match view_type {
            project_manager::ViewType::Window => self.window_update(view_id, |ctx, window| {
                let mut new_controls = window.controls.as_ref().clone();
                let changed = mutator(ctx, &mut new_controls)?;
                if changed {
                    window.controls = Arc::new(new_controls);
                }
                Ok(changed)
            }),
            project_manager::ViewType::Template => {
                self.template_update(view_id, |ctx, template| {
                    let mut new_controls = template.controls.as_ref().clone();
                    let changed = mutator(ctx, &mut new_controls)?;
                    if changed {
                        template.controls = Arc::new(new_controls);
                    }
                    Ok(changed)
                })
            }
        }
    }

    fn view_get_control(
        &self,
        view_type: project_manager::ViewType,
        view_id: &str,
        id: &str,
    ) -> Result<&Arc<project_manager::UiControlData>, ProjectManagerActorError> {
        match view_type {
            project_manager::ViewType::Window => {
                let window = self.window_get(view_id)?;
                let Some(control) = window.controls.get(id) else {
                    return Err(ProjectManagerActorError::DataProcessingError(format!(
                        "Control '{}' does not exist in window '{}'",
                        id, view_id
                    )));
                };

                Ok(control)
            }
            project_manager::ViewType::Template => {
                let template = self.template_get(view_id)?;
                let Some(control) = template.controls.get(id) else {
                    return Err(ProjectManagerActorError::DataProcessingError(format!(
                        "Control '{}' does not exist in template '{}'",
                        id, view_id
                    )));
                };

                Ok(control)
            }
        }
    }

    fn view_update_template_instances(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        mutator: impl FnOnce(
            &ProjectOperation,
            &mut HashMap<String, Arc<project_manager::UiTemplateInstanceData>>,
        ) -> Result<bool, ProjectManagerActorError>,
    ) -> Result<(), ProjectManagerActorError> {
        match view_type {
            project_manager::ViewType::Window => self.window_update(view_id, |ctx, window| {
                let mut new_templates = window.templates.as_ref().clone();
                let changed = mutator(ctx, &mut new_templates)?;
                if changed {
                    window.templates = Arc::new(new_templates);
                }
                Ok(changed)
            }),
            project_manager::ViewType::Template => {
                self.template_update(view_id, |ctx, template| {
                    let mut new_templates = template.templates.as_ref().clone();
                    let changed = mutator(ctx, &mut new_templates)?;
                    if changed {
                        template.templates = Arc::new(new_templates);
                    }
                    Ok(changed)
                })
            }
        }
    }

    fn view_get_template_instance(
        &self,
        view_type: project_manager::ViewType,
        view_id: &str,
        id: &str,
    ) -> Result<&Arc<project_manager::UiTemplateInstanceData>, ProjectManagerActorError> {
        match view_type {
            project_manager::ViewType::Window => {
                let window = self.window_get(view_id)?;
                let Some(template_instance) = window.templates.get(id) else {
                    return Err(ProjectManagerActorError::DataProcessingError(format!(
                        "Template instance '{}' does not exist in window '{}'",
                        id, view_id
                    )));
                };

                Ok(template_instance)
            }
            project_manager::ViewType::Template => {
                let template = self.template_get(view_id)?;
                let Some(template_instance) = template.templates.get(id) else {
                    return Err(ProjectManagerActorError::DataProcessingError(format!(
                        "Template instance '{}' does not exist in template '{}'",
                        id, view_id
                    )));
                };

                Ok(template_instance)
            }
        }
    }
}
