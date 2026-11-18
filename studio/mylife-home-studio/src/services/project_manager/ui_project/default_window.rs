use std::{collections::HashMap, sync::Arc};

use studio_web_api::{project_manager, ui_model};

use crate::services::project_manager::ui_project::validation::{
    InvalidForDeployment, Validation, ValidationResult,
};

pub trait DefaultWindowOps {
    fn set(
        &self,
        default_window: project_manager::UiDefaultWindowData,
    ) -> Arc<project_manager::UiDefaultWindowData>;

    fn on_change_window(
        &self,
        window_id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiDefaultWindowData>>;

    fn deploy(&self, validation: &Validation) -> ValidationResult<ui_model::DefaultWindow>;
}

impl DefaultWindowOps for Arc<project_manager::UiDefaultWindowData> {
    fn set(
        &self,
        default_window: project_manager::UiDefaultWindowData,
    ) -> Arc<project_manager::UiDefaultWindowData> {
        let mut data = self.0.clone();
        data.extend(default_window.0);
        Arc::new(project_manager::UiDefaultWindowData(data))
    }

    fn on_change_window(
        &self,
        window_id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiDefaultWindowData>> {
        let mut data = self.0.clone();
        let mut changed = false;

        for current_id in data.values_mut() {
            if current_id.as_deref() == Some(window_id) {
                *current_id = new_id.map(str::to_owned);
                changed = true;
            }
        }

        changed.then(|| Arc::new(project_manager::UiDefaultWindowData(data)))
    }

    fn deploy(&self, validation: &Validation) -> ValidationResult<ui_model::DefaultWindow> {
        let mut map = HashMap::new();
        let mut has_errors = false;

        for (typ, window_id) in &self.0 {
            let validation = validation.at("default-window", typ);

            let Ok(window_id) = validation.validate_window_id(window_id) else {
                has_errors = true;
                continue;
            };

            map.insert(typ.clone(), window_id);
        }

        if has_errors {
            Err(InvalidForDeployment)
        } else {
            Ok(ui_model::DefaultWindow(map))
        }
    }
}
