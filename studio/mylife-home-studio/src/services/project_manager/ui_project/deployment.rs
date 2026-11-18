use std::{collections::HashMap, sync::Arc};

use studio_web_api::{project_manager, ui_model};

use super::{
    control::ControlOps, default_window::DefaultWindowOps, template_instance::TemplateInstanceOps,
    validation::Validation,
};

pub trait DeploymentOps {
    fn prepare_deployment(
        &self,
    ) -> Result<ui_model::Definition, Vec<project_manager::UiValidationError>>;
}

impl DeploymentOps for Arc<project_manager::UiProject> {
    fn prepare_deployment(
        &self,
    ) -> Result<ui_model::Definition, Vec<project_manager::UiValidationError>> {
        let validation = Validation::new(self.clone());
        let default_window = self.default_window.deploy(&validation);

        for (template_id, template) in self.templates.iter() {
            let template_validation = validation
                .at("template", template_id)
                .with_owner_template(Some(template.clone()));
            validate_view(&template.controls, &template.templates, template_validation);
        }

        for (window_id, window) in self.windows.iter() {
            let mut window_validation = validation.at("window", window_id);
            window_validation
                .validate_optional_resource(&window.background_resource)
                .ok();
            validate_view(&window.controls, &window.templates, window_validation);
        }

        let errors = validation.errors();
        if !errors.is_empty() || default_window.is_err() {
            return Err(errors);
        }
        let default_window = match default_window {
            Ok(default_window) => default_window,
            Err(_) => return Err(errors),
        };

        let mut windows = Vec::with_capacity(self.windows.len());
        for (window_id, window) in self.windows.iter() {
            let controls = deploy_view(
                &window.controls,
                &window.templates,
                validation.at("window", window_id),
                &[],
                0,
                0,
            );
            windows.push(ui_model::Window {
                id: window_id.clone(),
                style: window.style.clone(),
                height: window.height,
                width: window.width,
                background_resource: window.background_resource.clone(),
                controls,
            });
        }

        let errors = validation.errors();
        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(ui_model::Definition {
            resources: self
                .resources
                .iter()
                .map(|(id, resource)| ui_model::DefinitionResource {
                    id: id.clone(),
                    mime: resource.mime.clone(),
                    data: resource.data.clone(),
                })
                .collect(),
            styles: self
                .styles
                .iter()
                .map(|(id, style)| ui_model::DefinitionStyle {
                    id: id.clone(),
                    properties: style.properties.clone(),
                })
                .collect(),
            windows,
            default_window,
        })
    }
}

fn validate_view(
    controls: &HashMap<String, Arc<project_manager::UiControlData>>,
    instances: &HashMap<String, Arc<project_manager::UiTemplateInstanceData>>,
    validation: Validation,
) {
    for (instance_id, instance) in instances {
        let validation = validation.at("template-instance", instance_id);
        validation.validate_id(&Some(instance_id.clone())).ok();
        let _ = instance.deploy(&validation);
    }

    for (control_id, control) in controls {
        let validation = validation.at("control", control_id);
        validation.validate_id(&Some(control_id.clone())).ok();
        let _ = control.deploy(control_id.clone(), &validation);
    }
}

fn deploy_view(
    controls: &HashMap<String, Arc<project_manager::UiControlData>>,
    instances: &HashMap<String, Arc<project_manager::UiTemplateInstanceData>>,
    validation: Validation,
    path: &[String],
    x_base: i32,
    y_base: i32,
) -> Vec<ui_model::Control> {
    let mut output = Vec::new();

    for (instance_id, instance) in instances {
        let validation = validation.at("template-instance", instance_id);
        if instance_id.trim().is_empty() {
            validation.add_error("L'identifiant est vide.");
        }

        let Ok((template, bindings)) = instance.deploy(&validation) else {
            continue;
        };

        let mut instance_path = path.to_vec();
        instance_path.push(instance_id.clone());
        let validation = validation
            .with_owner_template(Some(template.clone()))
            .with_bindings(bindings);
        output.extend(deploy_view(
            &template.controls,
            &template.templates,
            validation,
            &instance_path,
            x_base + instance.x,
            y_base + instance.y,
        ));
    }

    for (control_id, control) in controls {
        let validation = validation.at("control", control_id);
        if control_id.trim().is_empty() {
            let mut validation = validation.clone();
            validation.add_error("L'identifiant est vide.");
        }

        let Ok(mut deployed) = control.deploy(control_id.clone(), &validation) else {
            continue;
        };

        let mut control_path = path.to_vec();
        control_path.push(control_id.clone());
        deployed.id = control_path.join(":");
        deployed.x += x_base;
        deployed.y += y_base;
        output.push(deployed);
    }

    output
}
