use std::sync::Arc;

use studio_web_api::{project_manager, utils::PartialField};

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{resource::ResourceOps, style::StyleOps},
};

pub trait ControlOps {
    fn new(x: i32, y: i32, r#type: project_manager::UiControlType) -> Self;

    fn set_properties(
        &self,
        properties: project_manager::UiControlProperties,
    ) -> Option<Arc<project_manager::UiControlData>>;

    fn on_change_resource(
        &self,
        resource_id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiControlData>>;

    fn on_change_style(
        &self,
        style_id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiControlData>>;

    fn on_change_window(
        &self,
        window_id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiControlData>>;

    fn on_clear_template_export(
        &self,
        member_name: &str,
    ) -> Option<Arc<project_manager::UiControlData>>;

    fn clear_component_usage(
        &self,
        component_id: &str,
        member_name: &str,
    ) -> Option<Arc<project_manager::UiControlData>>;
}

impl ControlOps for Arc<project_manager::UiControlData> {
    fn new(x: i32, y: i32, r#type: project_manager::UiControlType) -> Self {
        let (display, text) = match r#type {
            project_manager::UiControlType::Display => (
                Some(Arc::new(project_manager::ControlDisplay {
                    component_id: None,
                    component_state: None,
                    default_resource: None,
                    map: Vec::new(),
                })),
                None,
            ),
            project_manager::UiControlType::Text => (
                None,
                Some(Arc::new(project_manager::UiControlTextData {
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
            primary_action: None,
            secondary_action: None,
        };

        Arc::new(control)
    }

    fn set_properties(
        &self,
        properties: project_manager::UiControlProperties,
    ) -> Option<Arc<project_manager::UiControlData>> {
        let mut control = self.as_ref().clone();
        let mut changed = false;

        if let PartialField::Value(mut style) = properties.style {
            style.normalize();
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
            control.display = display.map(Arc::new);
            changed = true;
        }

        if let PartialField::Value(text) = properties.text {
            control.text = text.map(Arc::new);
            changed = true;
        }

        if let PartialField::Value(primary_action) = properties.primary_action {
            control.primary_action = primary_action.map(Arc::new);
            changed = true;
        }

        if let PartialField::Value(secondary_action) = properties.secondary_action {
            control.secondary_action = secondary_action.map(Arc::new);
            changed = true;
        }

        if changed {
            Some(Arc::new(control))
        } else {
            None
        }
    }

    fn on_change_resource(
        &self,
        resource_id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiControlData>> {
        let Some(display) = &self.display else {
            return None;
        };

        let Some(display) = display.on_change_resource(resource_id, new_id) else {
            return None;
        };

        let mut control = self.as_ref().clone();
        control.display = Some(display);
        Some(Arc::new(control))
    }

    fn on_change_style(
        &self,
        style_id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiControlData>> {
        let Some(style) = self.style.on_change(style_id, new_id) else {
            return None;
        };

        let mut control = self.as_ref().clone();
        control.style = style;
        Some(Arc::new(control))
    }

    fn on_change_window(
        &self,
        window_id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiControlData>> {
        let mut primary_action = None;
        if let Some(action) = &self.primary_action {
            primary_action = action.on_change_window(window_id, new_id);
        }

        let mut secondary_action = None;
        if let Some(action) = &self.secondary_action {
            secondary_action = action.on_change_window(window_id, new_id);
        }

        if primary_action.is_none() && secondary_action.is_none() {
            return None;
        }

        let mut control = self.as_ref().clone();
        if let Some(action) = primary_action {
            control.primary_action = Some(action);
        }
        if let Some(action) = secondary_action {
            control.secondary_action = Some(action);
        }

        Some(Arc::new(control))
    }

    fn on_clear_template_export(
        &self,
        member_name: &str,
    ) -> Option<Arc<project_manager::UiControlData>> {
        let mut display = None;
        if let Some(disp) = &self.display {
            display = disp.on_clear_template_export(member_name);
        }

        let mut text = None;
        if let Some(txt) = &self.text {
            text = txt.on_clear_template_export(member_name);
        }

        let mut primary_action = None;
        if let Some(action) = &self.primary_action {
            primary_action = action.on_clear_template_export(member_name);
        }

        let mut secondary_action = None;
        if let Some(action) = &self.secondary_action {
            secondary_action = action.on_clear_template_export(member_name);
        }

        if display.is_none()
            && text.is_none()
            && primary_action.is_none()
            && secondary_action.is_none()
        {
            return None;
        }

        let mut control = self.as_ref().clone();
        if let Some(disp) = display {
            control.display = Some(disp);
        }
        if let Some(txt) = text {
            control.text = Some(txt);
        }
        if let Some(action) = primary_action {
            control.primary_action = Some(action);
        }
        if let Some(action) = secondary_action {
            control.secondary_action = Some(action);
        }

        Some(Arc::new(control))
    }

    fn clear_component_usage(
        &self,
        component_id: &str,
        member_name: &str,
    ) -> Option<Arc<project_manager::UiControlData>> {
        let mut display = None;
        if let Some(disp) = &self.display {
            display = disp.clear_component_usage(component_id, member_name);
        }

        let mut text = None;
        if let Some(txt) = &self.text {
            text = txt.clear_component_usage(component_id, member_name);
        }

        let mut primary_action = None;
        if let Some(action) = &self.primary_action {
            primary_action = action.clear_component_usage(component_id, member_name);
        }

        let mut secondary_action = None;
        if let Some(action) = &self.secondary_action {
            secondary_action = action.clear_component_usage(component_id, member_name);
        }

        if display.is_none()
            && text.is_none()
            && primary_action.is_none()
            && secondary_action.is_none()
        {
            return None;
        }

        let mut control = self.as_ref().clone();
        if let Some(disp) = display {
            control.display = Some(disp);
        }
        if let Some(txt) = text {
            control.text = Some(txt);
        }
        if let Some(action) = primary_action {
            control.primary_action = Some(action);
        }
        if let Some(action) = secondary_action {
            control.secondary_action = Some(action);
        }

        Some(Arc::new(control))
    }
}

pub trait ControlDisplayOps {
    fn on_change_resource(
        &self,
        resource_id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::ControlDisplay>>;

    fn on_clear_template_export(
        &self,
        member_name: &str,
    ) -> Option<Arc<project_manager::ControlDisplay>>;

    fn clear_component_usage(
        &self,
        component_id: &str,
        member_name: &str,
    ) -> Option<Arc<project_manager::ControlDisplay>>;
}

impl ControlDisplayOps for Arc<project_manager::ControlDisplay> {
    fn on_change_resource(
        &self,
        resource_id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::ControlDisplay>> {
        let mut display = self.as_ref().clone();
        let mut changed = false;

        if let Some(res) = &self.default_resource
            && res.as_str() == resource_id
        {
            display.default_resource = new_id.map(|id| project_manager::Resource::from_str(id));
            changed = true;
        }

        for item in &mut display.map {
            if let Some(res) = &item.resource
                && res.as_str() == resource_id
            {
                item.resource = new_id.map(|id| project_manager::Resource::from_str(id));
                changed = true;
            }
        }

        if changed {
            Some(Arc::new(display))
        } else {
            None
        }
    }

    fn on_clear_template_export(
        &self,
        member_name: &str,
    ) -> Option<Arc<project_manager::ControlDisplay>> {
        if self.component_id.is_none()
            && let Some(component_state) = &self.component_state
            && component_state == member_name
        {
            let mut display = self.as_ref().clone();
            display.component_state = None;
            Some(Arc::new(display))
        } else {
            None
        }
    }

    fn clear_component_usage(
        &self,
        component_id: &str,
        member_name: &str,
    ) -> Option<Arc<project_manager::ControlDisplay>> {
        if self.component_id.as_deref() == Some(component_id)
            && self.component_state.as_deref() == Some(member_name)
        {
            let mut display = self.as_ref().clone();
            display.component_id = None;
            display.component_state = None;
            Some(Arc::new(display))
        } else {
            None
        }
    }
}

pub trait ControlTextOps {
    fn on_clear_template_export(
        &self,
        member_name: &str,
    ) -> Option<Arc<project_manager::UiControlTextData>>;

    fn clear_component_usage(
        &self,
        component_id: &str,
        member_name: &str,
    ) -> Option<Arc<project_manager::UiControlTextData>>;
}

impl ControlTextOps for Arc<project_manager::UiControlTextData> {
    fn on_clear_template_export(
        &self,
        member_name: &str,
    ) -> Option<Arc<project_manager::UiControlTextData>> {
        let mut text = self.as_ref().clone();
        let mut changed = false;

        for item in &mut text.context {
            if item.component_id.is_none()
                && let Some(component_state) = &item.component_state
                && component_state == member_name
            {
                item.component_state = None;
                changed = true;
            }
        }

        if changed { Some(Arc::new(text)) } else { None }
    }

    fn clear_component_usage(
        &self,
        component_id: &str,
        member_name: &str,
    ) -> Option<Arc<project_manager::UiControlTextData>> {
        let mut text = self.as_ref().clone();
        let mut changed = false;

        for item in &mut text.context {
            if item.component_id.as_deref() == Some(component_id)
                && item.component_state.as_deref() == Some(member_name)
            {
                item.component_id = None;
                item.component_state = None;
                changed = true;
            }
        }

        if changed { Some(Arc::new(text)) } else { None }
    }
}

pub trait ActionOps {
    fn on_change_window(
        &self,
        window_id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiActionData>>;

    fn on_clear_template_export(
        &self,
        member_name: &str,
    ) -> Option<Arc<project_manager::UiActionData>>;

    fn clear_component_usage(
        &self,
        component_id: &str,
        member_name: &str,
    ) -> Option<Arc<project_manager::UiActionData>>;
}

impl ActionOps for Arc<project_manager::UiActionData> {
    fn on_change_window(
        &self,
        window_id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiActionData>> {
        if let Some(window) = &self.window
            && window.id.as_deref() == Some(window_id)
        {
            let mut action = self.as_ref().clone();
            action.window.as_mut().expect("data inconsistency").id = new_id.map(String::from);
            Some(Arc::new(action))
        } else {
            None
        }
    }

    fn on_clear_template_export(
        &self,
        member_name: &str,
    ) -> Option<Arc<project_manager::UiActionData>> {
        if let Some(component) = &self.component
            && component.id.is_none()
            && component.action.as_deref() == Some(member_name)
        {
            let mut action = self.as_ref().clone();
            action
                .component
                .as_mut()
                .expect("data inconsistency")
                .action = None;
            Some(Arc::new(action))
        } else {
            None
        }
    }

    fn clear_component_usage(
        &self,
        component_id: &str,
        member_name: &str,
    ) -> Option<Arc<project_manager::UiActionData>> {
        let mut action = self.as_ref().clone();
        let Some(component) = &mut action.component else {
            return None;
        };

        if component.id.as_deref() == Some(component_id)
            && component.action.as_deref() == Some(member_name)
        {
            let mut action = self.as_ref().clone();
            let component = action.component.as_mut().expect("data inconsistency");
            component.id = None;
            component.action = None;
            Some(Arc::new(action))
        } else {
            None
        }
    }
}
