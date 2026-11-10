use std::{collections::HashMap, sync::Arc, todo};

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{
        utils::{ArcMapOps, check_id},
        view::View,
        window::WindowOps,
    },
};

pub trait ProjectOps {
    fn set_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        resource: project_manager::UiResourceData,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn clear_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn rename_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn set_resources(
        &self,
        resources: Arc<HashMap<String, Arc<project_manager::UiResourceData>>>,
    ) -> Arc<project_manager::UiProject>;

    fn set_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        style: project_manager::UiStyleData,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn clear_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn rename_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn set_styles(
        &self,
        styles: Arc<HashMap<String, Arc<project_manager::UiStyleData>>>,
    ) -> Arc<project_manager::UiProject>;

    fn new_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn clear_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn rename_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn clone_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn set_window_properties(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        properties: project_manager::UiWindowProperties,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn set_windows(
        &self,
        windows: Arc<HashMap<String, Arc<project_manager::UiWindowData>>>,
    ) -> Arc<project_manager::UiProject>;

    fn get_view(
        &self,
        view_type: project_manager::ViewType,
        view_id: &str,
    ) -> Result<View, ProjectManagerActorError>;

    fn update_view(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        view_id: String,
        view: View,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn on_change_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError>;

    fn on_change_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError>;

    fn on_change_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError>;
}

impl ProjectOps for Arc<project_manager::UiProject> {
    fn set_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        resource: project_manager::UiResourceData,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        check_id(&id)?;
        let resource = Arc::new(resource);
        let resources = self.resources.with_set(id.clone(), resource.clone())?;
        let project = self.set_resources(resources);

        notifications.push(project_manager::UpdateProjectNotification::SetUiResource(
            project_manager::SetUiResourceNotification { id, resource },
        ));

        Ok(project)
    }

    fn clear_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let Some(resources) = self.resources.with_cleared(&id)? else {
            return Ok(None);
        };
        let project = self.set_resources(resources);

        notifications.push(project_manager::UpdateProjectNotification::ClearUiResource(
            project_manager::ClearUiResourceNotification { id: id.clone() },
        ));

        self.on_change_resource(notifications, &id, None)?;

        Ok(Some(project))
    }

    fn rename_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let Some(resources) = self.resources.with_renamed(&id, &new_id)? else {
            return Ok(None);
        };
        let project = self.set_resources(resources);

        notifications.push(
            project_manager::UpdateProjectNotification::RenameUiResource(
                project_manager::RenameUiResourceNotification {
                    id: id.clone(),
                    new_id: new_id.clone(),
                },
            ),
        );

        self.on_change_resource(notifications, &id, Some(&new_id))?;

        Ok(Some(project))
    }

    fn set_resources(
        &self,
        resources: Arc<HashMap<String, Arc<project_manager::UiResourceData>>>,
    ) -> Arc<project_manager::UiProject> {
        let mut project = self.as_ref().clone();
        project.resources = resources;
        Arc::new(project)
    }

    fn set_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        style: project_manager::UiStyleData,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        check_id(&id)?;
        let style = Arc::new(style);
        let styles = self.styles.with_set(id.clone(), style.clone())?;
        let project = self.set_styles(styles);

        notifications.push(project_manager::UpdateProjectNotification::SetUiStyle(
            project_manager::SetUiStyleNotification { id, style },
        ));

        Ok(project)
    }

    fn clear_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let Some(styles) = self.styles.with_cleared(&id)? else {
            return Ok(None);
        };
        let project = self.set_styles(styles);

        notifications.push(project_manager::UpdateProjectNotification::ClearUiStyle(
            project_manager::ClearUiStyleNotification { id: id.clone() },
        ));

        self.on_change_style(notifications, &id, None)?;

        Ok(Some(project))
    }

    fn rename_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let Some(styles) = self.styles.with_renamed(&id, &new_id)? else {
            return Ok(None);
        };
        let project = self.set_styles(styles);

        notifications.push(project_manager::UpdateProjectNotification::RenameUiStyle(
            project_manager::RenameUiStyleNotification {
                id: id.clone(),
                new_id: new_id.clone(),
            },
        ));

        self.on_change_style(notifications, &id, Some(&new_id))?;

        Ok(Some(project))
    }

    fn set_styles(
        &self,
        styles: Arc<HashMap<String, Arc<project_manager::UiStyleData>>>,
    ) -> Arc<project_manager::UiProject> {
        let mut project = self.as_ref().clone();
        project.styles = styles;
        Arc::new(project)
    }

    fn new_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let window = <Arc<project_manager::UiWindowData> as WindowOps>::new();
        let windows = self.windows.with_inserted(id.clone(), window.clone())?;
        let project = self.set_windows(windows);

        notifications.push(project_manager::UpdateProjectNotification::SetUiWindow(
            project_manager::SetUiWindowNotification { id, window },
        ));

        Ok(project)
    }

    fn clear_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let Some(windows) = self.windows.with_cleared(&id)? else {
            return Ok(None);
        };
        let project = self.set_windows(windows);

        notifications.push(project_manager::UpdateProjectNotification::ClearUiWindow(
            project_manager::ClearUiWindowNotification { id: id.clone() },
        ));

        self.on_change_window(notifications, &id, None)?;

        Ok(Some(project))
    }

    fn rename_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let Some(windows) = self.windows.with_renamed(&id, &new_id)? else {
            return Ok(None);
        };
        let project = self.set_windows(windows);

        notifications.push(project_manager::UpdateProjectNotification::RenameUiWindow(
            project_manager::RenameUiWindowNotification {
                id: id.clone(),
                new_id: new_id.clone(),
            },
        ));

        self.on_change_window(notifications, &id, Some(&new_id))?;

        Ok(Some(project))
    }

    fn clone_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let window = self.windows.get_required(&id)?.clone();
        let windows = self.windows.with_inserted(new_id.clone(), window.clone())?;
        let project = self.set_windows(windows);

        notifications.push(project_manager::UpdateProjectNotification::SetUiWindow(
            project_manager::SetUiWindowNotification { id: new_id, window },
        ));

        Ok(project)
    }

    fn set_window_properties(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        properties: project_manager::UiWindowProperties,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let window = self.windows.get_required(&id)?;
        let Some(window) = window.set_properties(properties) else {
            return Ok(None);
        };
        let windows = self.windows.with_updated(id.clone(), window.clone())?;
        let project = self.set_windows(windows);

        notifications.push(project_manager::UpdateProjectNotification::SetUiWindow(
            project_manager::SetUiWindowNotification { id, window },
        ));

        Ok(Some(project))
    }

    fn set_windows(
        &self,
        windows: Arc<HashMap<String, Arc<project_manager::UiWindowData>>>,
    ) -> Arc<project_manager::UiProject> {
        let mut project = self.as_ref().clone();
        project.windows = windows;
        Arc::new(project)
    }

    fn get_view(
        &self,
        view_type: project_manager::ViewType,
        view_id: &str,
    ) -> Result<View, ProjectManagerActorError> {
        match view_type {
            project_manager::ViewType::Window => {
                let window = self.windows.get_required(view_id)?;
                Ok(View::Window(window.clone()))
            }
            project_manager::ViewType::Template => {
                let template = self.templates.get_required(view_id)?;
                Ok(View::Template(template.clone()))
            }
        }
    }

    fn update_view(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        view_id: String,
        view: View,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        match view {
            View::Window(window) => {
                let windows = self.windows.with_updated(view_id.clone(), window.clone())?;
                let mut project = self.as_ref().clone();
                project.windows = windows;
                let project = Arc::new(project);

                notifications.push(project_manager::UpdateProjectNotification::SetUiWindow(
                    project_manager::SetUiWindowNotification {
                        id: view_id,
                        window,
                    },
                ));

                Ok(project)
            }
            View::Template(template) => {
                let templates = self
                    .templates
                    .with_updated(view_id.clone(), template.clone())?;
                let mut project = self.as_ref().clone();
                project.templates = templates;
                let project = Arc::new(project);

                notifications.push(project_manager::UpdateProjectNotification::SetUiTemplate(
                    project_manager::SetUiTemplateNotification {
                        id: view_id,
                        template,
                    },
                ));

                Ok(project)
            }
        }
    }

    fn on_change_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    fn on_change_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }

    fn on_change_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError> {
        todo!()
    }
}
