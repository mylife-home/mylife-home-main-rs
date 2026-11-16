use std::{collections::HashMap, sync::Arc};

use studio_web_api::project_manager;

use crate::services::project_manager::ui_project::{
    control::ControlOps, resource::OptionResourceOps, style::StyleOps,
    template_instance::TemplateInstanceOps, utils::ArcMapOps,
};

pub enum View {
    Window(Arc<project_manager::UiWindowData>),
    Template(Arc<project_manager::UiTemplateData>),
}

impl View {
    pub fn window(window: Arc<project_manager::UiWindowData>) -> View {
        View::Window(window)
    }

    pub fn template(template: Arc<project_manager::UiTemplateData>) -> View {
        View::Template(template)
    }

    pub fn r#type(&self) -> project_manager::ViewType {
        match self {
            View::Window(_) => project_manager::ViewType::Window,
            View::Template(_) => project_manager::ViewType::Template,
        }
    }

    pub fn as_window(&self) -> Option<&Arc<project_manager::UiWindowData>> {
        match self {
            View::Window(window) => Some(window),
            _ => None,
        }
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

                let mut updated = window.as_ref().clone();
                if let Some(controls) = controls {
                    updated.controls = controls;
                }
                if let Some(background_resource) = background_resource {
                    updated.background_resource = background_resource;
                }
                Some(View::window(Arc::new(updated)))
            }
            View::Template(template) => {
                let controls = template
                    .controls
                    .with_values_updated(|_, control| control.on_change_resource(id, new_id))?;
                let mut updated = template.as_ref().clone();
                updated.controls = controls;
                Some(View::template(Arc::new(updated)))
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

                let mut updated = window.as_ref().clone();
                if let Some(controls) = controls {
                    updated.controls = controls;
                }
                if let Some(style) = style {
                    updated.style = style;
                }
                Some(View::window(Arc::new(updated)))
            }
            View::Template(template) => {
                let controls = template
                    .controls
                    .with_values_updated(|_, control| control.on_change_style(id, new_id))?;
                let mut updated = template.as_ref().clone();
                updated.controls = controls;
                Some(View::template(Arc::new(updated)))
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
}
