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

    fn control_on_change_resource(
        &self,
        control: &mut project_manager::UiControlData,
        resource_id: &str,
        new_id: Option<&str>,
    ) -> Result<bool, ProjectManagerActorError>;
    fn control_on_change_style(
        &self,
        control: &mut project_manager::UiControlData,
        style_id: &str,
        new_id: Option<&str>,
    ) -> Result<bool, ProjectManagerActorError>;
    fn control_on_change_window(
        &self,
        control: &mut project_manager::UiControlData,
        window_id: &str,
        new_id: Option<&str>,
    ) -> Result<bool, ProjectManagerActorError>;
    fn control_on_clear_template_export(
        &self,
        control: &mut project_manager::UiControlData,
        member_name: &str,
    ) -> Result<bool, ProjectManagerActorError>;
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

        if id == new_id {
            return Ok(());
        }

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

    fn control_on_change_resource(
        &self,
        control: &mut project_manager::UiControlData,
        resource_id: &str,
        new_id: Option<&str>,
    ) -> Result<bool, ProjectManagerActorError> {
        let Some(mut display) = control.display.as_ref().clone() else {
            return Ok(false);
        };

        let mut changed = false;

        if let Some(res) = &display.default_resource
            && res.0 == resource_id
        {
            display.default_resource = new_id.map(|id| project_manager::Resource(String::from(id)));
            changed = true;
        }

        for item in &mut display.map {
            if let Some(res) = &item.resource
                && res.0 == resource_id
            {
                item.resource = new_id.map(|id| project_manager::Resource(String::from(id)));
                changed = true;
            }
        }

        if changed {
            control.display = Arc::new(Some(display));
        }

        Ok(changed)
    }

    fn control_on_change_style(
        &self,
        control: &mut project_manager::UiControlData,
        style_id: &str,
        new_id: Option<&str>,
    ) -> Result<bool, ProjectManagerActorError> {
        let list = &mut control.style.0;

        let Some(index) = list.iter().position(|item| item == style_id) else {
            return Ok(false);
        };

        if let Some(new_id) = new_id {
            list[index] = String::from(new_id);
        } else {
            list.remove(index);
        }

        Ok(true)
    }

    fn control_on_change_window(
        &self,
        control: &mut project_manager::UiControlData,
        window_id: &str,
        new_id: Option<&str>,
    ) -> Result<bool, ProjectManagerActorError> {
        let update = |action: &mut project_manager::UiActionData| -> bool {
            if let Some(window) = &mut action.window
                && let Some(id) = &window.id
                && id == window_id
            {
                window.id = new_id.map(String::from);
                true
            } else {
                false
            }
        };

        let mut changed = false;

        if let Some(mut action) = control.primary_action.as_ref().clone() {
            let changed_primary = update(&mut action);
            if changed_primary {
                control.primary_action = Arc::new(Some(action));
            }
            changed |= changed_primary;
        }

        if let Some(mut action) = control.secondary_action.as_ref().clone() {
            let changed_sec = update(&mut action);
            if changed_sec {
                control.secondary_action = Arc::new(Some(action));
            }
            changed |= changed_sec;
        }

        Ok(changed)
    }

    fn control_on_clear_template_export(
        &self,
        control: &mut project_manager::UiControlData,
        member_name: &str,
    ) -> Result<bool, ProjectManagerActorError> {
        let mut changed = false;

        if let Some(mut display) = control.display.as_ref().clone() {
            if display.component_id.is_none()
                && let Some(component_state) = &display.component_state
                && component_state == member_name
            {
                display.component_state = None;
                control.display = Arc::new(Some(display));
                changed = true;
            }
        }

        if let Some(mut text) = control.text.as_ref().clone() {
            let mut change_text = false;
            for item in &mut text.context {
                if item.component_id.is_none()
                    && let Some(component_state) = &item.component_state
                    && component_state == member_name
                {
                    item.component_state = None;
                    change_text = true;
                }
            }

            if change_text {
                control.text = Arc::new(Some(text));
            }
            changed |= change_text;
        }

        let update = |action: &mut project_manager::UiActionData| -> bool {
            if let Some(component) = &mut action.component
                && component.id.is_none()
                && let Some(action) = &component.action
                && action == member_name
            {
                component.action = None;
                true
            } else {
                false
            }
        };

        if let Some(mut action) = control.primary_action.as_ref().clone() {
            let changed_primary = update(&mut action);
            if changed_primary {
                control.primary_action = Arc::new(Some(action));
            }
            changed |= changed_primary;
        }

        if let Some(mut action) = control.secondary_action.as_ref().clone() {
            let changed_sec = update(&mut action);
            if changed_sec {
                control.secondary_action = Arc::new(Some(action));
            }
            changed |= changed_sec;
        }

        Ok(changed)
    }
}
