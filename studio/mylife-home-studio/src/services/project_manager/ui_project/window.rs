use std::{collections::HashMap, sync::Arc};

use studio_web_api::{project_manager, utils::PartialField};

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{control::ControlOperation, operation::ProjectOperation},
};

pub trait WindowOperation {
    /// Note: self.data() will not be modified by this function
    fn window_initial_notifications(&mut self);

    fn window_new(&mut self, id: String) -> Result<(), ProjectManagerActorError>;
    fn window_clear(&mut self, id: String) -> Result<(), ProjectManagerActorError>;
    fn window_rename(&mut self, id: String, new_id: String)
    -> Result<(), ProjectManagerActorError>;
    fn window_clone(&mut self, id: String, new_id: String) -> Result<(), ProjectManagerActorError>;
    fn window_set_properties(
        &mut self,
        id: String,
        properties: project_manager::UiWindowProperties,
    ) -> Result<(), ProjectManagerActorError>;
    fn window_update(
        &mut self,
        id: String,
        mutator: impl FnOnce(
            &ProjectOperation,
            &mut project_manager::UiWindowData,
        ) -> Result<bool, ProjectManagerActorError>,
    ) -> Result<(), ProjectManagerActorError>;
    fn window_get(
        &self,
        id: &str,
    ) -> Result<&Arc<project_manager::UiWindowData>, ProjectManagerActorError>;

    fn window_on_change_resource(
        &mut self,
        resource_id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError>;
    fn window_on_change_style(
        &mut self,
        style_id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError>;
}

impl WindowOperation for ProjectOperation {
    fn window_initial_notifications(&mut self) {
        for (id, window) in self.data().windows.clone().iter() {
            self.push_notification(project_manager::UpdateProjectNotification::SetUiWindow(
                project_manager::SetUiWindowNotification {
                    id: id.clone(),
                    window: window.clone(),
                },
            ));
        }

        for (id, template) in self.data().templates.clone().iter() {
            self.push_notification(project_manager::UpdateProjectNotification::SetUiTemplate(
                project_manager::SetUiTemplateNotification {
                    id: id.clone(),
                    template: template.clone(),
                },
            ));
        }
    }

    fn window_new(&mut self, id: String) -> Result<(), ProjectManagerActorError> {
        self.check_id(&id)?;

        if self.data().windows.contains_key(&id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Window with id '{}' already exists",
                id
            )));
        }

        let window = Arc::new(project_manager::UiWindowData {
            style: project_manager::Style(vec![]),
            height: 500,
            width: 500,
            background_resource: None,
            controls: Arc::new(HashMap::new()),
            templates: Arc::new(HashMap::new()),
        });

        let mut new_windows = self.data().windows.as_ref().clone();
        new_windows.insert(id.clone(), window.clone());

        self.set_data(Arc::new(project_manager::UiProject {
            windows: Arc::new(new_windows),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(project_manager::UpdateProjectNotification::SetUiWindow(
            project_manager::SetUiWindowNotification { id, window },
        ));

        Ok(())
    }

    fn window_clear(&mut self, id: String) -> Result<(), ProjectManagerActorError> {
        let mut new_windows = self.data().windows.as_ref().clone();
        if new_windows.remove(&id).is_none() {
            return Ok(());
        }

        self.set_data(Arc::new(project_manager::UiProject {
            windows: Arc::new(new_windows),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(project_manager::UpdateProjectNotification::ClearUiWindow(
            project_manager::ClearUiWindowNotification { id },
        ));

        // TODO: impacts

        Ok(())
    }

    fn window_rename(
        &mut self,
        id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.check_id(&new_id)?;

        if id == new_id {
            return Ok(());
        }

        if self.data().windows.contains_key(&new_id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Window with id '{}' already exists",
                new_id
            )));
        }

        let mut new_windows = self.data().windows.as_ref().clone();
        let Some(window) = new_windows.remove(&id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Window with id '{}' does not exist",
                id
            )));
        };

        new_windows.insert(new_id.clone(), window);

        self.set_data(Arc::new(project_manager::UiProject {
            windows: Arc::new(new_windows),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(project_manager::UpdateProjectNotification::RenameUiWindow(
            project_manager::RenameUiWindowNotification { id, new_id },
        ));

        // TODO: impacts

        Ok(())
    }

    fn window_clone(&mut self, id: String, new_id: String) -> Result<(), ProjectManagerActorError> {
        self.check_id(&new_id)?;

        let mut new_windows = self.data().windows.as_ref().clone();
        let Some(window) = new_windows.get(&id).cloned() else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Window with id '{}' does not exist",
                id
            )));
        };

        if new_windows.contains_key(&new_id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Window with id '{}' already exists",
                new_id
            )));
        }

        new_windows.insert(new_id.clone(), window.clone());

        self.set_data(Arc::new(project_manager::UiProject {
            windows: Arc::new(new_windows),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(project_manager::UpdateProjectNotification::SetUiWindow(
            project_manager::SetUiWindowNotification { id: new_id, window },
        ));

        Ok(())
    }

    fn window_set_properties(
        &mut self,
        id: String,
        properties: project_manager::UiWindowProperties,
    ) -> Result<(), ProjectManagerActorError> {
        self.window_update(id, |_, window| {
            let mut changed = false;

            if let PartialField::Value(height) = properties.height {
                window.height = height;
                changed = true;
            }
            if let PartialField::Value(width) = properties.width {
                window.width = width;
                changed = true;
            }
            if let PartialField::Value(style) = properties.style {
                window.style = style;
                changed = true;
            }
            if let PartialField::Value(background_resource) = properties.background_resource {
                window.background_resource = background_resource;
                changed = true;
            }

            Ok(changed)
        })
    }

    fn window_update(
        &mut self,
        id: String,
        mutator: impl FnOnce(
            &ProjectOperation,
            &mut project_manager::UiWindowData,
        ) -> Result<bool, ProjectManagerActorError>,
    ) -> Result<(), ProjectManagerActorError> {
        let mut new_windows = self.data().windows.as_ref().clone();
        let Some(window) = new_windows.get(&id).cloned() else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Window with id '{}' does not exist",
                id
            )));
        };

        let mut window = window.as_ref().clone();
        let changed = mutator(self, &mut window)?;

        if !changed {
            return Ok(());
        }

        let window = Arc::new(window);
        new_windows.insert(id.clone(), window.clone());

        self.set_data(Arc::new(project_manager::UiProject {
            windows: Arc::new(new_windows),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(project_manager::UpdateProjectNotification::SetUiWindow(
            project_manager::SetUiWindowNotification { id, window },
        ));

        Ok(())
    }

    fn window_get(
        &self,
        id: &str,
    ) -> Result<&Arc<project_manager::UiWindowData>, ProjectManagerActorError> {
        let Some(window) = self.data().windows.get(id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Window '{}' does not exist",
                id
            )));
        };

        Ok(window)
    }

    fn window_on_change_resource(
        &mut self,
        resource_id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError> {
        for window_id in self.data().windows.keys().cloned().collect::<Vec<_>>() {
            self.window_update(window_id, |self_, window| {
                let mut changed = false;

                if let Some(background_resource) = &window.background_resource
                    && background_resource.0 == resource_id
                {
                    window.background_resource =
                        new_id.map(|id| project_manager::Resource(String::from(id)));
                    changed = true;
                }

                let mut new_controls = window.controls.as_ref().clone();
                let mut changed_controls = false;
                for (_, control) in &mut new_controls.iter_mut() {
                    let mut new_control = control.as_ref().clone();
                    if self_.control_on_change_resource(&mut new_control, resource_id, new_id)? {
                        *control = Arc::new(new_control);
                        changed_controls = true;
                    }
                }

                if changed_controls {
                    window.controls = Arc::new(new_controls);
                }
                changed |= changed_controls;

                Ok(changed)
            })?;
        }

        Ok(())
    }

    fn window_on_change_style(
        &mut self,
        style_id: &str,
        new_id: Option<&str>,
    ) -> Result<(), ProjectManagerActorError> {
        for window_id in self.data().windows.keys().cloned().collect::<Vec<_>>() {
            self.window_update(window_id, |self_, window| {
                let mut changed = false;

                let list = &mut window.style.0;
                if let Some(index) = list.iter().position(|item| item == style_id) {
                    if let Some(new_id) = new_id {
                        list[index] = String::from(new_id);
                    } else {
                        list.remove(index);
                    }

                    changed = true;
                }

                let mut new_controls = window.controls.as_ref().clone();
                let mut changed_controls = false;
                for (_, control) in &mut new_controls.iter_mut() {
                    let mut new_control = control.as_ref().clone();
                    if self_.control_on_change_style(&mut new_control, style_id, new_id)? {
                        *control = Arc::new(new_control);
                        changed_controls = true;
                    }
                }

                if changed_controls {
                    window.controls = Arc::new(new_controls);
                }
                changed |= changed_controls;

                Ok(changed)
            })?;
        }

        Ok(())
    }
}
