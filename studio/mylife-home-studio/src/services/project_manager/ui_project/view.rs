use std::{collections::HashMap, sync::Arc};

use studio_web_api::project_manager;

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

    pub fn get_controls(&self) -> &HashMap<String, Arc<project_manager::UiControlData>> {
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
}
