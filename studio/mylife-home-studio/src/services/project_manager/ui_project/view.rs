use std::{collections::HashMap, sync::Arc};

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{
        control::ControlOperation, operation::ProjectOperation, template::TemplateOperation,
        window::WindowOperation,
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
    fn view_on_change_template(
        &mut self,
        template_id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError>;
    fn view_on_change_template_export(
        &mut self,
        template_id: &str,
        export_id: &str,
        set: bool,
    ) -> Result<(), ProjectManagerActorError>;
    fn view_on_change_resource(
        &mut self,
        resource_id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError>;
    fn view_on_change_style(
        &mut self,
        style_id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError>;
    fn view_on_change_window(
        &mut self,
        window_id: &str,
        new_id: Option<&str>,
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

    fn view_on_change_template(
        &mut self,
        template_id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError> {
        let window_ids = self.data().windows.keys().cloned().collect::<Vec<_>>();
        let template_ids = self.data().templates.keys().cloned().collect::<Vec<_>>();

        for (view_type, view_ids) in [
            (project_manager::ViewType::Window, window_ids),
            (project_manager::ViewType::Template, template_ids),
        ] {
            for view_id in view_ids {
                self.view_update_template_instances(view_type, view_id, |_, instances| {
                    let mut changed = false;
                    let mut removed = Vec::new();

                    for (id, instance) in instances.iter_mut() {
                        if instance.template_id.as_str() != template_id {
                            continue;
                        }

                        if let Some(new_id) = new_id {
                            let mut updated = instance.as_ref().clone();
                            updated.template_id = new_id.to_owned();
                            *instance = Arc::new(updated);
                        } else {
                            removed.push(id.clone());
                        }
                        changed = true;
                    }

                    for id in removed {
                        instances.remove(&id);
                    }

                    Ok(changed)
                })?;
            }
        }

        Ok(())
    }

    fn view_on_change_template_export(
        &mut self,
        template_id: &str,
        export_id: &str,
        set: bool,
    ) -> Result<(), ProjectManagerActorError> {
        let window_ids = self.data().windows.keys().cloned().collect::<Vec<_>>();
        let template_ids = self.data().templates.keys().cloned().collect::<Vec<_>>();

        for (view_type, view_ids) in [
            (project_manager::ViewType::Window, window_ids),
            (project_manager::ViewType::Template, template_ids),
        ] {
            for view_id in view_ids {
                self.view_update_template_instances(view_type, view_id, |_, instances| {
                    let mut changed = false;

                    for instance in instances.values_mut() {
                        if instance.template_id.as_str() != template_id {
                            continue;
                        }

                        let mut updated = instance.as_ref().clone();
                        let mut bindings = updated.bindings.as_ref().clone();
                        let binding = if set {
                            bindings.insert(
                                export_id.to_owned(),
                                Arc::new(project_manager::UiTemplateInstanceBinding {
                                    component_id: None,
                                    member_name: None,
                                }),
                            );
                            true
                        } else {
                            bindings.remove(export_id).is_some()
                        };

                        if binding {
                            updated.bindings = Arc::new(bindings);
                            *instance = Arc::new(updated);
                            changed = true;
                        }
                    }

                    Ok(changed)
                })?;
            }
        }

        Ok(())
    }

    fn view_on_change_resource(
        &mut self,
        resource_id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError> {
        let window_ids = self.data().windows.keys().cloned().collect::<Vec<_>>();
        for window_id in window_ids {
            self.window_update(window_id, |operation, window| {
                let mut changed = false;

                if window
                    .background_resource
                    .as_ref()
                    .is_some_and(|resource| resource.0.as_str() == resource_id)
                {
                    window.background_resource =
                        new_id.map(|id| project_manager::Resource(id.to_owned()));
                    changed = true;
                }

                let mut controls = window.controls.as_ref().clone();
                for control in controls.values_mut() {
                    let mut updated = control.as_ref().clone();
                    if operation.control_on_change_resource(&mut updated, resource_id, new_id)? {
                        *control = Arc::new(updated);
                        changed = true;
                    }
                }

                if changed {
                    window.controls = Arc::new(controls);
                }
                Ok(changed)
            })?;
        }

        let template_ids = self.data().templates.keys().cloned().collect::<Vec<_>>();
        for template_id in template_ids {
            self.template_update(template_id, |operation, template| {
                let mut controls = template.controls.as_ref().clone();
                let mut changed = false;
                for control in controls.values_mut() {
                    let mut updated = control.as_ref().clone();
                    if operation.control_on_change_resource(&mut updated, resource_id, new_id)? {
                        *control = Arc::new(updated);
                        changed = true;
                    }
                }

                if changed {
                    template.controls = Arc::new(controls);
                }
                Ok(changed)
            })?;
        }

        Ok(())
    }

    fn view_on_change_style(
        &mut self,
        style_id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError> {
        let window_ids = self.data().windows.keys().cloned().collect::<Vec<_>>();
        for window_id in window_ids {
            self.window_update(window_id, |operation, window| {
                let mut changed = false;
                let styles = &mut window.style.0;
                if let Some(index) = styles.iter().position(|style| style.as_str() == style_id) {
                    if let Some(new_id) = new_id {
                        styles[index] = new_id.to_owned();
                        styles.sort();
                    } else {
                        styles.remove(index);
                    }
                    changed = true;
                }

                let mut controls = window.controls.as_ref().clone();
                for control in controls.values_mut() {
                    let mut updated = control.as_ref().clone();
                    if operation.control_on_change_style(&mut updated, style_id, new_id)? {
                        *control = Arc::new(updated);
                        changed = true;
                    }
                }

                if changed {
                    window.controls = Arc::new(controls);
                }
                Ok(changed)
            })?;
        }

        let template_ids = self.data().templates.keys().cloned().collect::<Vec<_>>();
        for template_id in template_ids {
            self.template_update(template_id, |operation, template| {
                let mut controls = template.controls.as_ref().clone();
                let mut changed = false;
                for control in controls.values_mut() {
                    let mut updated = control.as_ref().clone();
                    if operation.control_on_change_style(&mut updated, style_id, new_id)? {
                        *control = Arc::new(updated);
                        changed = true;
                    }
                }

                if changed {
                    template.controls = Arc::new(controls);
                }
                Ok(changed)
            })?;
        }

        Ok(())
    }

    fn view_on_change_window(
        &mut self,
        window_id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError> {
        let window_ids = self.data().windows.keys().cloned().collect::<Vec<_>>();
        let template_ids = self.data().templates.keys().cloned().collect::<Vec<_>>();

        for (view_type, view_ids) in [
            (project_manager::ViewType::Window, window_ids),
            (project_manager::ViewType::Template, template_ids),
        ] {
            for view_id in view_ids {
                self.view_update_controls(view_type, view_id, |operation, controls| {
                    let mut changed = false;
                    for control in controls.values_mut() {
                        let mut updated = control.as_ref().clone();
                        if operation.control_on_change_window(&mut updated, window_id, new_id)? {
                            *control = Arc::new(updated);
                            changed = true;
                        }
                    }
                    Ok(changed)
                })?;
            }
        }

        Ok(())
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
