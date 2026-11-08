use std::sync::Arc;

use studio_web_api::{project_manager, utils::PartialField};

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{operation::ProjectOperation, view::ViewOperation},
};

pub trait ControlOperation {
    fn control_new(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        x: i32,
        y: i32,
        r#type: project_manager::UiControlType,
    ) -> Result<(), ProjectManagerActorError>;
    fn control_clear(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
    ) -> Result<(), ProjectManagerActorError>;
    fn control_rename(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError>;
    fn control_clone(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        new_id: String,
        target_view_type: project_manager::ViewType,
        target_view_id: String,
    ) -> Result<(), ProjectManagerActorError>;
    fn control_set_properties(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        properties: project_manager::UiControlProperties,
    ) -> Result<(), ProjectManagerActorError>;
}

impl ControlOperation for ProjectOperation {
    fn control_new(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        x: i32,
        y: i32,
        r#type: project_manager::UiControlType,
    ) -> Result<(), ProjectManagerActorError> {
        self.check_id(&id)?;

        self.view_update_controls(view_type, view_id, |_, controls| {
            if controls.contains_key(&id) {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Control with id '{}' already exists",
                    id
                )));
            }

            let (display, text) = match r#type {
                project_manager::UiControlType::Display => (
                    Arc::new(Some(project_manager::ControlDisplay {
                        component_id: None,
                        component_state: None,
                        default_resource: None,
                        map: Vec::new(),
                    })),
                    Arc::new(None),
                ),
                project_manager::UiControlType::Text => (
                    Arc::new(None),
                    Arc::new(Some(project_manager::UiControlTextData {
                        context: Vec::new(),
                        format: String::from("return '';"),
                    })),
                ),
            };

            let control = project_manager::UiControlData {
                x,
                y,
                style: project_manager::Style(Vec::new()),
                height: 50,
                width: 50,
                display,
                text,
                primary_action: Arc::new(None),
                secondary_action: Arc::new(None),
            };

            controls.insert(id, Arc::new(control));
            Ok(true)
        })
    }

    fn control_clear(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.view_update_controls(view_type, view_id, |_, controls| {
            Ok(controls.remove(&id).is_some())
        })
    }

    fn control_rename(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.check_id(&new_id)?;

        self.view_update_controls(view_type, view_id, |_, controls| {
            if controls.contains_key(&new_id) {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Control with id '{}' already exists",
                    new_id
                )));
            }

            let Some(control) = controls.remove(&id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Control with id '{}' does not exist",
                    id
                )));
            };

            controls.insert(new_id, control);

            Ok(true)
        })
    }

    fn control_clone(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        new_id: String,
        target_view_type: project_manager::ViewType,
        target_view_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.check_id(&new_id)?;

        let same_view = view_type == target_view_type && view_id == target_view_id;
        let source_control = self.view_get_control(view_type, &view_id, &id)?;
        let mut new_control = source_control.as_ref().clone();

        if same_view {
            new_control.x += 10;
            new_control.y += 10;
        } else {
            new_control.x = 10;
            new_control.y = 10;
        }

        let new_control = Arc::new(new_control);

        self.view_update_controls(target_view_type, target_view_id, |_, controls| {
            if controls.contains_key(&new_id) {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Control with id '{}' already exists",
                    new_id
                )));
            }

            controls.insert(new_id, new_control);

            Ok(true)
        })
    }

    fn control_set_properties(
        &mut self,
        view_type: project_manager::ViewType,
        view_id: String,
        id: String,
        properties: project_manager::UiControlProperties,
    ) -> Result<(), ProjectManagerActorError> {
        self.view_update_controls(view_type, view_id, |_, controls| {
            let Some(control) = controls.get_mut(&id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Control with id '{}' does not exist",
                    id
                )));
            };

            let mut changed = false;
            let mut control = control.as_ref().clone();

            if let PartialField::Value(mut style) = properties.style {
                style.0.sort();
                control.style = style;
                changed = true;
            }
            if let PartialField::Value(height) = properties.height {
                control.height = height;
                changed = true;
            }
            if let PartialField::Value(width) = properties.width {
                control.width = width;
                changed = true;
            }
            if let PartialField::Value(x) = properties.x {
                control.x = x;
                changed = true;
            }
            if let PartialField::Value(y) = properties.y {
                control.y = y;
                changed = true;
            }
            if let PartialField::Value(display) = properties.display {
                control.display = Arc::new(display);
                changed = true;
            }
            if let PartialField::Value(text) = properties.text {
                control.text = Arc::new(text);
                changed = true;
            }
            if let PartialField::Value(primary_action) = properties.primary_action {
                control.primary_action = Arc::new(primary_action);
                changed = true;
            }
            if let PartialField::Value(secondary_action) = properties.secondary_action {
                control.secondary_action = Arc::new(secondary_action);
                changed = true;
            }

            if !changed {
                return Ok(false);
            }

            controls.insert(id, Arc::new(control));
            Ok(true)
        })
    }
}
