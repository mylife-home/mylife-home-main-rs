use std::{collections::HashMap, sync::Arc};

use studio_web_api::{project_manager, ui_model};

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{
        component::{ComponentUsage, ComponentUsageCollector},
        control::ControlOps,
        resource::OptionResourceOps,
        style::StyleOps,
        template_instance::TemplateInstanceOps,
        utils::ArcMapOps,
        validation::{InvalidForDeployment, Validation, ValidationResult},
    },
};

#[derive(Clone, Default)]
pub struct DeployPosition {
    pub prefix: String,
    pub x: i32,
    pub y: i32,
}

impl DeployPosition {
    pub fn child(&self, id: &str, x: i32, y: i32) -> Self {
        Self {
            prefix: if self.prefix.is_empty() {
                id.to_owned()
            } else {
                format!("{}:{}", self.prefix, id)
            },
            x: self.x + x,
            y: self.y + y,
        }
    }
}

#[derive(Debug, Clone)]
pub enum View {
    Window(Arc<project_manager::UiWindowData>),
    Template(Arc<project_manager::UiTemplateData>),
}

impl View {
    pub fn deploy_controls(
        &self,
        validation: &Validation,
        position: &DeployPosition,
    ) -> ValidationResult<Vec<ui_model::Control>> {
        let instances = self.get_templates().deploy(
            validation,
            "template-instance",
            |validation, id, instance| {
                let valid_id = validation.validate_id(&Some(id.clone()));
                let controls =
                    instance.deploy(validation, &position.child(id, instance.x, instance.y));

                let (Ok(_), Ok(controls)) = (valid_id, controls) else {
                    return Err(InvalidForDeployment);
                };

                Ok(controls)
            },
        );
        let controls =
            self.get_controls()
                .deploy(validation, "control", |validation, id, control| {
                    let valid_id = validation.validate_id(&Some(id.clone()));
                    let position = position.child(id, control.x, control.y);
                    let deployed = control.deploy(position.prefix, validation);

                    let (Ok(_), Ok(mut control)) = (valid_id, deployed) else {
                        return Err(InvalidForDeployment);
                    };

                    control.x = position.x;
                    control.y = position.y;
                    Ok(control)
                });
        match (instances, controls) {
            (Ok(instances), Ok(controls)) => {
                Ok(instances.into_iter().flatten().chain(controls).collect())
            }
            _ => Err(InvalidForDeployment),
        }
    }

    pub fn r#type(&self) -> project_manager::ViewType {
        match self {
            View::Window(_) => project_manager::ViewType::Window,
            View::Template(_) => project_manager::ViewType::Template,
        }
    }

    pub fn window(window: Arc<project_manager::UiWindowData>) -> View {
        View::Window(window)
    }

    pub fn template(template: Arc<project_manager::UiTemplateData>) -> View {
        View::Template(template)
    }

    pub fn as_template(&self) -> Option<&Arc<project_manager::UiTemplateData>> {
        match self {
            View::Template(template) => Some(template),
            _ => None,
        }
    }

    pub fn get_controls(&self) -> &Arc<HashMap<String, Arc<project_manager::UiControlData>>> {
        match self {
            View::Window(window) => &window.controls,
            View::Template(template) => &template.controls,
        }
    }

    pub fn set_controls(
        &self,
        controls: Arc<HashMap<String, Arc<project_manager::UiControlData>>>,
    ) -> View {
        match self {
            View::Window(window) => {
                let mut window = window.as_ref().clone();
                window.controls = controls;
                Self::window(Arc::new(window))
            }
            View::Template(template) => {
                let mut template = template.as_ref().clone();
                template.controls = controls;
                Self::template(Arc::new(template))
            }
        }
    }

    pub fn get_templates(
        &self,
    ) -> &Arc<HashMap<String, Arc<project_manager::UiTemplateInstanceData>>> {
        match self {
            View::Window(window) => &window.templates,
            View::Template(template) => &template.templates,
        }
    }

    pub fn set_templates(
        &self,
        templates: Arc<HashMap<String, Arc<project_manager::UiTemplateInstanceData>>>,
    ) -> View {
        match self {
            View::Window(window) => {
                let mut window = window.as_ref().clone();
                window.templates = templates;
                Self::window(Arc::new(window))
            }
            View::Template(template) => {
                let mut template = template.as_ref().clone();
                template.templates = templates;
                Self::template(Arc::new(template))
            }
        }
    }

    pub fn on_change_resource(&self, id: &str, new_id: Option<&str>) -> Option<View> {
        match self {
            View::Window(window) => {
                let controls = window
                    .controls
                    .with_values_updated(|_, control| control.on_change_resource(id, new_id));
                let background_resource = window.background_resource.on_change_resource(id, new_id);
                if controls.is_none() && background_resource.is_none() {
                    return None;
                }

                let mut window = window.as_ref().clone();
                if let Some(controls) = controls {
                    window.controls = controls;
                }
                if let Some(background_resource) = background_resource {
                    window.background_resource = background_resource;
                }
                Some(View::window(Arc::new(window)))
            }
            View::Template(template) => {
                let controls = template
                    .controls
                    .with_values_updated(|_, control| control.on_change_resource(id, new_id))?;
                let mut template = template.as_ref().clone();
                template.controls = controls;
                Some(View::template(Arc::new(template)))
            }
        }
    }

    pub fn on_change_style(&self, id: &str, new_id: Option<&str>) -> Option<View> {
        match self {
            View::Window(window) => {
                let controls = window
                    .controls
                    .with_values_updated(|_, control| control.on_change_style(id, new_id));
                let style = window.style.on_change(id, new_id);
                if controls.is_none() && style.is_none() {
                    return None;
                }

                let mut window = window.as_ref().clone();
                if let Some(controls) = controls {
                    window.controls = controls;
                }
                if let Some(style) = style {
                    window.style = style;
                }
                Some(View::window(Arc::new(window)))
            }
            View::Template(template) => {
                let controls = template
                    .controls
                    .with_values_updated(|_, control| control.on_change_style(id, new_id))?;
                let mut template = template.as_ref().clone();
                template.controls = controls;
                Some(View::template(Arc::new(template)))
            }
        }
    }

    pub fn on_change_window(&self, id: &str, new_id: Option<&str>) -> Option<View> {
        let controls = self
            .get_controls()
            .with_values_updated(|_, control| control.on_change_window(id, new_id))?;
        Some(self.set_controls(controls))
    }

    pub fn on_change_template(&self, id: &str, new_id: Option<&str>) -> Option<View> {
        let templates = self.get_templates();
        let updated_templates = if let Some(new_id) = new_id {
            templates.with_values_updated(|_, instance| instance.on_rename_template(id, new_id))
        } else {
            let mut updated = templates.as_ref().clone();
            updated.retain(|_, instance| instance.template_id != id);
            (updated.len() != templates.len()).then(|| Arc::new(updated))
        }?;

        Some(self.set_templates(updated_templates))
    }

    pub fn collect_component_usage(&self, collector: &ComponentUsageCollector) {
        for (id, control) in self.get_controls().iter() {
            let collector = collector.at("control", id);
            control.collect_component_usage(&collector);
        }

        for (id, template) in self.get_templates().iter() {
            let collector = collector.at("template-instance", id);
            template.collect_component_usage(&collector);
        }
    }

    pub fn clear_component_usage(
        &self,
        usage: &ComponentUsage,
    ) -> Result<Option<View>, ProjectManagerActorError> {
        let Some(node) = usage.path.get(1) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Malformed usage path: {:?}",
                usage
            )));
        };

        let view = match node.r#type.as_str() {
            "control" => {
                let controls = self.get_controls();
                let control = controls.get_required(&node.id)?;
                if let Some(control) = control.clear_component_usage(usage) {
                    let controls = controls.with_updated(node.id.clone(), control)?;
                    Some(self.set_controls(controls))
                } else {
                    None
                }
            }
            "template-instance" => {
                let templates = self.get_templates();
                let template = templates.get_required(&node.id)?;
                if let Some(template) = template.clear_component_usage(usage) {
                    let templates = templates.with_updated(node.id.clone(), template)?;
                    Some(self.set_templates(templates))
                } else {
                    None
                }
            }
            _ => {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Unknown view element type: {:?}",
                    node.r#type
                )));
            }
        };

        Ok(view)
    }
}
