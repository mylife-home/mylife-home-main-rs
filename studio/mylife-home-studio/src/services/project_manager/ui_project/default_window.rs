use std::sync::Arc;

use studio_web_api::project_manager;

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
}
