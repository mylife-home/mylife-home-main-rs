use std::sync::Arc;

use studio_web_api::project_manager;

#[derive(Debug)]
pub struct ProjectModel {
    data: Arc<project_manager::UiProject>,
    default_window: DefaultWindowModel,
}

impl ProjectModel {
    pub fn new(data: &Arc<project_manager::UiProject>) -> Self {
        Self {
            default_window: DefaultWindowModel::new(&data.default_window),
            data: data.clone(),
        }
    }

    pub fn initial_notifications(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
    ) {
        notifications.push(
            project_manager::UpdateProjectNotification::SetUiDefaultWindow(
                project_manager::SetUiDefaultWindowNotification {
                    default_window: self.default_window.data.as_ref().clone(),
                },
            ),
        );
        /*

            notifier.notify({ operation: 'set-ui-default-window', defaultWindow: this.project.defaultWindow } as SetUiDefaultWindowNotification);
            notifier.notify({ operation: 'set-ui-component-data', components: this.project.components, plugins: this.project.plugins } as SetUiComponentDataNotification);

            for (const [id, resource] of Object.entries(this.project.resources)) {
              notifier.notify({ operation: 'set-ui-resource', id, resource } as SetUiResourceNotification);
            }

            for (const [id, style] of Object.entries(this.project.styles)) {
              notifier.notify({ operation: 'set-ui-style', id, style } as SetUiStyleNotification);
            }

            for (const [id, window] of Object.entries(this.project.windows)) {
              notifier.notify({ operation: 'set-ui-window', id, window } as SetUiWindowNotification);
            }

            for (const [id, template] of Object.entries(this.project.templates)) {
              notifier.notify({ operation: 'set-ui-template', id, template } as SetUiTemplateNotification);
            }
        */
    }

    pub fn data(&self) -> &Arc<project_manager::UiProject> {
        &self.data
    }

    pub fn set_default_window(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        data: project_manager::UiDefaultWindowData,
    ) {
        self.default_window.set(notifications, data);

        self.data = Arc::new(project_manager::UiProject {
            default_window: self.default_window.data.clone(),
            ..(*self.data).clone()
        });
    }
}

#[derive(Debug)]
struct DefaultWindowModel {
    data: Arc<project_manager::UiDefaultWindowData>,
}

impl DefaultWindowModel {
    pub fn new(data: &Arc<project_manager::UiDefaultWindowData>) -> Self {
        Self { data: data.clone() }
    }

    pub fn set(
        &mut self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        data: project_manager::UiDefaultWindowData,
    ) {
        let mut new_map = self.data.0.clone();

        for (key, value) in data.0 {
            new_map.insert(key.clone(), value);
        }

        self.data = Arc::new(project_manager::UiDefaultWindowData(new_map));

        notifications.push(
            project_manager::UpdateProjectNotification::SetUiDefaultWindow(
                project_manager::SetUiDefaultWindowNotification {
                    default_window: self.data.as_ref().clone(),
                },
            ),
        );
    }
}
