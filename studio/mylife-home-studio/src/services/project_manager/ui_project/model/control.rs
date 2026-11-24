//! The control tree: a control, the two halves it can be, and the actions it
//! can trigger.
//!
//! These live apart from `types.rs` because they are what the reference walks
//! operate on: four separate implementations of "drop every reference to this"
//! and "collect every reference from this", which only agree with each other
//! if they can be read together.

use std::sync::Arc;

use super::ids::{ExportId, ResourceId, StyleId, WindowId};
use super::path::{ElementPath, PathNode};
use super::properties::ControlProperties;
use super::types::{MemberRef, StyleList};
use super::usage::{ComponentUsage, UsageCollector};

#[derive(Debug, Clone)]
pub enum ControlKind {
    Display,
    Text,
}

#[derive(Debug, Clone)]
pub struct Control {
    pub style: StyleList,
    pub height: i32,
    pub width: i32,
    pub x: i32,
    pub y: i32,
    pub display: Option<Arc<ControlDisplay>>,
    pub text: Option<Arc<ControlText>>,
    pub primary_action: Option<Arc<Action>>,
    pub secondary_action: Option<Arc<Action>>,
}

impl Control {
    pub fn new(x: i32, y: i32, kind: ControlKind) -> Self {
        let (display, text) = match kind {
            ControlKind::Display => (Some(Arc::new(ControlDisplay::empty())), None),
            ControlKind::Text => (None, Some(Arc::new(ControlText::empty()))),
        };

        Self {
            style: StyleList::default(),
            height: 50,
            width: 50,
            x,
            y,
            display,
            text,
            primary_action: None,
            secondary_action: None,
        }
    }

    /// A copy. Offset from the original when it lands in the same view, placed
    /// at a fixed spot when it lands in another one.
    pub fn cloned_at(&self, same_view: bool) -> Arc<Self> {
        let mut control = self.clone();

        if same_view {
            control.x += 10;
            control.y += 10;
        } else {
            control.x = 10;
            control.y = 10;
        }

        Arc::new(control)
    }

    pub fn with_properties(&self, properties: ControlProperties) -> Option<Arc<Self>> {
        let mut control = self.clone();
        let mut changed = false;

        changed |= properties.style.apply_to(&mut control.style);
        changed |= properties.height.apply_to(&mut control.height);
        changed |= properties.width.apply_to(&mut control.width);
        changed |= properties.x.apply_to(&mut control.x);
        changed |= properties.y.apply_to(&mut control.y);
        changed |= properties.display.apply_to(&mut control.display);
        changed |= properties.text.apply_to(&mut control.text);
        changed |= properties
            .primary_action
            .apply_to(&mut control.primary_action);
        changed |= properties
            .secondary_action
            .apply_to(&mut control.secondary_action);

        changed.then(|| Arc::new(control))
    }
}

impl Control {
    pub fn on_change_resource(
        &self,
        id: &ResourceId,
        new_id: Option<&ResourceId>,
    ) -> Option<Arc<Self>> {
        let display = self
            .display
            .as_ref()
            .and_then(|display| display.on_change_resource(id, new_id));

        display.map(|display| {
            let mut control = self.clone();
            control.display = Some(display);
            Arc::new(control)
        })
    }

    pub fn on_change_style(&self, id: &StyleId, new_id: Option<&StyleId>) -> Option<Arc<Self>> {
        let style = self.style.on_change_style(id, new_id)?;

        let mut control = self.clone();
        control.style = style;
        Some(Arc::new(control))
    }

    pub fn on_change_window(&self, id: &WindowId, new_id: Option<&WindowId>) -> Option<Arc<Self>> {
        let primary = self
            .primary_action
            .as_ref()
            .and_then(|action| action.on_change_window(id, new_id));
        let secondary = self
            .secondary_action
            .as_ref()
            .and_then(|action| action.on_change_window(id, new_id));

        if primary.is_none() && secondary.is_none() {
            return None;
        }

        let mut control = self.clone();
        if let Some(action) = primary {
            control.primary_action = Some(action);
        }
        if let Some(action) = secondary {
            control.secondary_action = Some(action);
        }
        Some(Arc::new(control))
    }

    /// An export of the enclosing template has gone; drop the references to it.
    ///
    /// This and `clear_component_usage` below walk the same four places for the
    /// same four references, and differ only in which `MemberRef` variant they
    /// are looking for: the enum arm is the whole of the distinction between
    /// them.
    pub fn on_clear_template_export(&self, export_id: &ExportId) -> Option<Arc<Self>> {
        let display = self
            .display
            .as_ref()
            .and_then(|display| display.on_clear_template_export(export_id));
        let text = self
            .text
            .as_ref()
            .and_then(|text| text.on_clear_template_export(export_id));
        let primary_action = self
            .primary_action
            .as_ref()
            .and_then(|action| action.on_clear_template_export(export_id));
        let secondary_action = self
            .secondary_action
            .as_ref()
            .and_then(|action| action.on_clear_template_export(export_id));

        if display.is_none()
            && text.is_none()
            && primary_action.is_none()
            && secondary_action.is_none()
        {
            return None;
        }

        let mut control = self.clone();
        if let Some(display) = display {
            control.display = Some(display);
        }
        if let Some(text) = text {
            control.text = Some(text);
        }
        if let Some(action) = primary_action {
            control.primary_action = Some(action);
        }
        if let Some(action) = secondary_action {
            control.secondary_action = Some(action);
        }
        Some(Arc::new(control))
    }

    /// The path grows one node per step down, so a usage says *which* reference
    /// it came from and not just which control.
    pub fn collect_component_usage(&self, path: &ElementPath, collector: &mut UsageCollector) {
        let mut push = |path: ElementPath, member: &Option<MemberRef>| {
            if let Some((component_id, name)) = member.as_ref().and_then(MemberRef::component_usage)
            {
                collector.push(component_id, name, &path);
            }
        };

        if let Some(display) = &self.display {
            push(path.child(PathNode::Display), &display.member);
        }

        if let Some(text) = &self.text {
            let text_path = path.child(PathNode::Text);

            for (index, item) in text.context.iter().enumerate() {
                push(text_path.child(PathNode::ContextItem(index)), &item.member);
            }
        }

        for (node, action) in [
            (PathNode::PrimaryAction, &self.primary_action),
            (PathNode::SecondaryAction, &self.secondary_action),
        ] {
            if let Some(action) = action
                && let Some(component) = &action.component
            {
                push(path.child(node), &component.member);
            }
        }
    }

    pub fn clear_component_usage(&self, usage: &ComponentUsage) -> Option<Arc<Self>> {
        let display = self
            .display
            .as_ref()
            .and_then(|display| display.clear_component_usage(usage));
        let text = self
            .text
            .as_ref()
            .and_then(|text| text.clear_component_usage(usage));
        let primary_action = self
            .primary_action
            .as_ref()
            .and_then(|action| action.clear_component_usage(usage));
        let secondary_action = self
            .secondary_action
            .as_ref()
            .and_then(|action| action.clear_component_usage(usage));

        if display.is_none()
            && text.is_none()
            && primary_action.is_none()
            && secondary_action.is_none()
        {
            return None;
        }

        let mut control = self.clone();
        if let Some(display) = display {
            control.display = Some(display);
        }
        if let Some(text) = text {
            control.text = Some(text);
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

#[derive(Debug, Clone)]
pub struct ControlDisplay {
    /// `None` while nothing is bound. Inside a template this can be an export
    /// of the enclosing template rather than a component.
    pub member: Option<MemberRef>,
    pub default_resource: Option<ResourceId>,
    pub map: Vec<ControlDisplayMapItem>,
}

impl ControlDisplay {
    pub fn empty() -> Self {
        Self {
            member: None,
            default_resource: None,
            map: Vec::new(),
        }
    }
}

impl ControlDisplay {
    pub fn on_change_resource(
        &self,
        id: &ResourceId,
        new_id: Option<&ResourceId>,
    ) -> Option<Arc<Self>> {
        let mut display = self.clone();
        let mut changed = false;

        if self.default_resource.as_ref() == Some(id) {
            display.default_resource = new_id.cloned();
            changed = true;
        }

        for item in &mut display.map {
            if item.resource.as_ref() == Some(id) {
                item.resource = new_id.cloned();
                changed = true;
            }
        }

        changed.then(|| Arc::new(display))
    }

    pub fn on_clear_template_export(&self, export_id: &ExportId) -> Option<Arc<Self>> {
        if !self
            .member
            .as_ref()
            .is_some_and(|member| member.is_owner_export(export_id))
        {
            return None;
        }

        let mut display = self.clone();
        display.member = None;
        Some(Arc::new(display))
    }

    pub fn clear_component_usage(&self, usage: &ComponentUsage) -> Option<Arc<Self>> {
        if !usage.matches(&self.member) {
            return None;
        }

        let mut display = self.clone();
        display.member = None;
        Some(Arc::new(display))
    }
}

#[derive(Debug, Clone)]
pub struct ControlDisplayMapItem {
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub value: serde_json::Value,
    pub resource: Option<ResourceId>,
}

#[derive(Debug, Clone)]
pub struct ControlText {
    pub context: Vec<ControlTextContextItem>,
    pub format: String,
}

impl ControlText {
    pub fn empty() -> Self {
        Self {
            context: Vec::new(),
            format: String::from("return '';"),
        }
    }
}

impl ControlText {
    pub fn on_clear_template_export(&self, export_id: &ExportId) -> Option<Arc<Self>> {
        let mut text = self.clone();
        let mut changed = false;

        for item in &mut text.context {
            if item
                .member
                .as_ref()
                .is_some_and(|member| member.is_owner_export(export_id))
            {
                item.member = None;
                changed = true;
            }
        }

        changed.then(|| Arc::new(text))
    }

    pub fn clear_component_usage(&self, usage: &ComponentUsage) -> Option<Arc<Self>> {
        let mut text = self.clone();
        let mut changed = false;

        for item in &mut text.context {
            if usage.matches(&item.member) {
                item.member = None;
                changed = true;
            }
        }

        changed.then(|| Arc::new(text))
    }
}

#[derive(Debug, Clone)]
pub struct ControlTextContextItem {
    pub id: String,
    pub member: Option<MemberRef>,
    /// Designer-only, not deployed.
    pub test_value: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct Action {
    /// `Some` means this is a component action; the reference inside it is
    /// `None` while that action is unbound. The two `None`s mean different
    /// things and sit at different depths, which is the only thing that tells
    /// them apart.
    pub component: Option<ActionComponent>,
    pub window: Option<ActionWindow>,
}

impl Action {
    pub fn on_change_window(&self, id: &WindowId, new_id: Option<&WindowId>) -> Option<Arc<Self>> {
        let window = self.window.as_ref()?;

        if window.id.as_ref() != Some(id) {
            return None;
        }

        let mut action = self.clone();
        if let Some(window) = &mut action.window {
            window.id = new_id.cloned();
        }
        Some(Arc::new(action))
    }

    pub fn on_clear_template_export(&self, export_id: &ExportId) -> Option<Arc<Self>> {
        let component = self.component.as_ref()?;

        if !component
            .member
            .as_ref()
            .is_some_and(|member| member.is_owner_export(export_id))
        {
            return None;
        }

        let mut action = self.clone();
        if let Some(component) = &mut action.component {
            component.member = None;
        }
        Some(Arc::new(action))
    }

    pub fn clear_component_usage(&self, usage: &ComponentUsage) -> Option<Arc<Self>> {
        let component = self.component.as_ref()?;

        if !usage.matches(&component.member) {
            return None;
        }

        let mut action = self.clone();
        if let Some(component) = &mut action.component {
            component.member = None;
        }
        Some(Arc::new(action))
    }
}

#[derive(Debug, Clone)]
pub struct ActionComponent {
    pub member: Option<MemberRef>,
}

#[derive(Debug, Clone)]
pub struct ActionWindow {
    /// `None` once the target window has been deleted.
    pub id: Option<WindowId>,
    pub popup: bool,
}
