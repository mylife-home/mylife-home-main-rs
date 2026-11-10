use std::sync::Arc;

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
}
