use std::collections::HashMap;
use std::sync::Arc;

use studio_web_api::component_model::{Member, MemberType};

use super::Collection;
use super::control::Control;
use super::error::UiModelError;
use super::ids::{
    ComponentId, ControlId, ExportId, PluginId, ResourceId, StyleId, TemplateId,
    TemplateInstanceId, WindowId,
};
use super::path::{ElementPath, PathNode};
use super::properties::{TemplateProperties, WindowProperties};
use super::usage::{ComponentUsage, UsageCollector};

pub const fn member_type_name(member_type: MemberType) -> &'static str {
    match member_type {
        MemberType::Action => "action",
        MemberType::State => "state",
    }
}

/// The CSS classes applied to a window or control.
///
/// On the wire this is `Style(Vec<String>)`. Here the entries are `StyleId`, so
/// a rename walk compares ids rather than strings and cannot be pointed at the
/// wrong kind of name. The list is sorted by its only constructor, so two style
/// lists with the same members compare equal and nothing has to remember to
/// normalize.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleList(Vec<StyleId>);

impl StyleList {
    pub fn new(mut styles: Vec<StyleId>) -> Self {
        styles.sort();
        Self(styles)
    }

    pub fn as_slice(&self) -> &[StyleId] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// `None` when this list does not mention `id`, so an untouched control
    /// keeps its pointer.
    pub fn on_change_style(&self, id: &StyleId, new_id: Option<&StyleId>) -> Option<Self> {
        let index = self.0.iter().position(|item| item == id)?;

        let mut styles = self.0.clone();

        match new_id {
            Some(new_id) => {
                styles[index] = new_id.clone();
                styles.sort();
            }
            None => {
                styles.remove(index);
            }
        }

        Some(Self(styles))
    }
}

/// What a control or a binding points at.
///
/// On the wire every one of these is a pair of `Option<String>`:
/// `(component_id, component_state)` on a display and on a text context item,
/// `(id, action)` on an action, `(component_id, member_name)` on a
/// template-instance binding. The pair encodes three states, and one that has
/// no meaning:
///
/// | component | member | means                                     |
/// |-----------|--------|-------------------------------------------|
/// | set       | set    | that member of that component             |
/// | unset     | set    | an export of the *enclosing* template     |
/// | unset     | unset  | not bound yet                             |
/// | set       | unset  | nothing; unreachable through the operations |
///
/// The second row is the one that costs: the string there is an export id, not
/// a member name, and nothing in the pair says so. As an enum each row is one
/// `match` arm, the export id has its own type, and the fourth row cannot be
/// built at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberRef {
    Component {
        id: ComponentId,
        member: String,
    },
    /// Only meaningful inside a template, which is why binding to one needs
    /// the enclosing view to be a template.
    OwnerExport(ExportId),
}

impl MemberRef {
    pub fn component(id: ComponentId, member: impl Into<String>) -> Self {
        Self::Component {
            id,
            member: member.into(),
        }
    }

    /// The component and member, when this is a component reference. This is
    /// the only thing the usage walk cares about.
    pub fn component_usage(&self) -> Option<(&ComponentId, &str)> {
        match self {
            MemberRef::Component { id, member } => Some((id, member.as_str())),
            MemberRef::OwnerExport(_) => None,
        }
    }

    pub fn owner_export(&self) -> Option<&ExportId> {
        match self {
            MemberRef::OwnerExport(id) => Some(id),
            MemberRef::Component { .. } => None,
        }
    }

    /// `true` when this points at `export_id` of the enclosing template, which
    /// is the whole condition `on_clear_template_export` tests.
    pub fn is_owner_export(&self, export_id: &ExportId) -> bool {
        self.owner_export() == Some(export_id)
    }
}

/// One binding slot: a template export, bound or not.
pub type Binding = Option<MemberRef>;

#[derive(Debug, Clone)]
pub struct Resource {
    pub mime: String,
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct StyleDefinition {
    pub properties: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct Component {
    /// Typed, so "which plugin does this component use" is answerable without
    /// guessing what the string means.
    pub plugin: PluginId,
}

#[derive(Debug, Clone)]
pub struct Plugin {
    pub name: String,
    pub module: String,
    pub version: String,
    pub description: Option<String>,
    pub members: HashMap<String, Member>,
    pub instance_name: String,
}

impl Plugin {
    pub fn member(&self, name: &str, member_type: MemberType) -> Option<&Member> {
        self.members
            .get(name)
            .filter(|member| member.member_type == member_type)
    }
}

/// Context key (`desktop`, `mobile`, ...) to the window shown by default.
#[derive(Debug, Clone, Default)]
pub struct DefaultWindow(HashMap<String, Option<WindowId>>);

impl DefaultWindow {
    pub fn new(entries: HashMap<String, Option<WindowId>>) -> Self {
        Self(entries)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Option<WindowId>)> {
        self.0.iter()
    }

    pub fn on_change_window(&self, id: &WindowId, new_id: Option<&WindowId>) -> Option<Self> {
        let mut entries: Option<HashMap<String, Option<WindowId>>> = None;

        for (key, window) in self.0.iter() {
            if window.as_ref() != Some(id) {
                continue;
            }

            entries
                .get_or_insert_with(|| self.0.clone())
                .insert(key.clone(), new_id.cloned());
        }

        entries.map(Self)
    }
}

#[derive(Debug, Clone)]
pub struct Window {
    pub height: i32,
    pub width: i32,
    pub controls: Collection<ControlId, Control>,
    pub templates: Collection<TemplateInstanceId, TemplateInstance>,
    pub style: StyleList,
    pub background_resource: Option<ResourceId>,
}

impl Window {
    pub fn empty() -> Self {
        Self {
            height: 500,
            width: 500,
            controls: Collection::new(),
            templates: Collection::new(),
            style: StyleList::default(),
            background_resource: None,
        }
    }

    pub fn with_properties(&self, properties: WindowProperties) -> Option<Arc<Self>> {
        let mut window = self.clone();
        let mut changed = false;

        changed |= properties.height.apply_to(&mut window.height);
        changed |= properties.width.apply_to(&mut window.width);
        changed |= properties.style.apply_to(&mut window.style);
        changed |= properties
            .background_resource
            .apply_to(&mut window.background_resource);

        changed.then(|| Arc::new(window))
    }
}

#[derive(Debug, Clone)]
pub struct Template {
    pub height: i32,
    pub width: i32,
    pub controls: Collection<ControlId, Control>,
    pub templates: Collection<TemplateInstanceId, TemplateInstance>,
    pub exports: Collection<ExportId, TemplateExport>,
}

impl Template {
    pub fn empty() -> Self {
        Self {
            height: 500,
            width: 500,
            controls: Collection::new(),
            templates: Collection::new(),
            exports: Collection::new(),
        }
    }

    pub fn with_properties(&self, properties: TemplateProperties) -> Option<Arc<Self>> {
        let mut template = self.clone();
        let mut changed = false;

        changed |= properties.height.apply_to(&mut template.height);
        changed |= properties.width.apply_to(&mut template.width);

        changed.then(|| Arc::new(template))
    }

    pub fn with_export(
        &self,
        id: ExportId,
        member_type: MemberType,
        value_type: impl Into<String>,
    ) -> Arc<Self> {
        let export = Arc::new(TemplateExport {
            bulk_pattern: None,
            description: None,
            member_type,
            value_type: value_type.into(),
        });

        let mut template = self.clone();
        template.exports = self.exports.set(id, export);
        Arc::new(template)
    }

    /// Drop an export and every reference to it from inside this template.
    ///
    /// Only this template can hold those references: an export is addressed
    /// from its own controls and from the instances nested in it. Instances of
    /// this template elsewhere keep a binding slot whose key no longer exists,
    /// which is the current behaviour and is caught at deploy time rather than
    /// here.
    pub fn without_export(&self, id: &ExportId) -> Option<Arc<Self>> {
        let exports = self.exports.removed(id)?;

        let mut template = self.clone();
        template.exports = exports;

        if let Some(controls) = self
            .controls
            .map_values(|_, control| control.on_clear_template_export(id))
        {
            template.controls = controls;
        }

        if let Some(instances) = self
            .templates
            .map_values(|_, instance| instance.on_clear_template_export(id))
        {
            template.templates = instances;
        }

        Some(Arc::new(template))
    }

    pub fn with_bulk_patterns(
        &self,
        patterns: &HashMap<ExportId, String>,
    ) -> Result<Option<Arc<Self>>, UiModelError> {
        let mut exports = self.exports.clone();
        let mut changed = false;

        for (id, pattern) in patterns {
            let mut export = exports.get_required(id)?.as_ref().clone();
            export.bulk_pattern = Some(pattern.clone());
            exports = exports.set(id.clone(), Arc::new(export));
            changed = true;
        }

        if !changed {
            return Ok(None);
        }

        let mut template = self.clone();
        template.exports = exports;
        Ok(Some(Arc::new(template)))
    }

    /// One unbound slot per export, which is what a fresh instance starts from
    /// and what re-pointing an instance at another template resets it to.
    pub fn prepare_bindings(&self) -> HashMap<ExportId, Binding> {
        self.exports.keys().map(|id| (id.clone(), None)).collect()
    }

    pub fn export_value_type(
        &self,
        id: &ExportId,
        member_type: MemberType,
    ) -> Result<&str, UiModelError> {
        let export = self.exports.get_required(id)?;

        if export.member_type != member_type {
            return Err(UiModelError::ExportMemberTypeMismatch {
                id: id.to_string(),
                expected: member_type_name(export.member_type),
                actual: member_type_name(member_type),
            });
        }

        Ok(export.value_type.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct TemplateExport {
    pub bulk_pattern: Option<String>,
    pub description: Option<String>,
    pub member_type: MemberType,
    pub value_type: String,
}

#[derive(Debug, Clone)]
pub struct TemplateInstance {
    pub template_id: TemplateId,
    pub x: i32,
    pub y: i32,
    pub bindings: HashMap<ExportId, Binding>,
}

impl TemplateInstance {
    pub fn new(template_id: TemplateId, template: &Template, x: i32, y: i32) -> Self {
        Self {
            template_id,
            x,
            y,
            bindings: template.prepare_bindings(),
        }
    }

    /// A copy offset from the original, as the designer's "duplicate" does.
    pub fn cloned_nearby(&self) -> Arc<Self> {
        let mut instance = self.clone();
        instance.x += 10;
        instance.y += 10;
        Arc::new(instance)
    }

    pub fn moved(&self, x: i32, y: i32) -> Arc<Self> {
        let mut instance = self.clone();
        instance.x = x;
        instance.y = y;
        Arc::new(instance)
    }

    /// Re-point at another template. Bindings cannot survive, since they are
    /// keyed by the old template's exports.
    pub fn with_template(&self, template_id: TemplateId, template: &Template) -> Arc<Self> {
        Arc::new(Self {
            template_id,
            x: self.x,
            y: self.y,
            bindings: template.prepare_bindings(),
        })
    }

    pub fn with_bindings(&self, bindings: HashMap<ExportId, Binding>) -> Arc<Self> {
        let mut instance = self.clone();
        instance.bindings = bindings;
        Arc::new(instance)
    }
}

impl TemplateInstance {
    pub fn collect_component_usage(&self, path: &ElementPath, collector: &mut UsageCollector) {
        for (export_id, binding) in &self.bindings {
            if let Some((component_id, member)) =
                binding.as_ref().and_then(MemberRef::component_usage)
            {
                collector.push(
                    component_id,
                    member,
                    &path.child(PathNode::Binding(export_id.clone())),
                );
            }
        }
    }

    pub fn clear_component_usage(&self, usage: &ComponentUsage) -> Option<Arc<Self>> {
        let mut instance = self.clone();
        let mut changed = false;

        for binding in instance.bindings.values_mut() {
            if usage.matches(binding) {
                *binding = None;
                changed = true;
            }
        }

        changed.then(|| Arc::new(instance))
    }

    /// An export of the enclosing template has gone. Only a binding that points
    /// at the enclosing template can be affected; one bound to a component
    /// cannot, and does not have to be told apart by hand.
    pub fn on_clear_template_export(&self, export_id: &ExportId) -> Option<Arc<Self>> {
        let mut instance = self.clone();
        let mut changed = false;

        for binding in instance.bindings.values_mut() {
            if binding
                .as_ref()
                .is_some_and(|member| member.is_owner_export(export_id))
            {
                *binding = None;
                changed = true;
            }
        }

        changed.then(|| Arc::new(instance))
    }

    pub fn on_change_template(&self, id: &TemplateId, new_id: &TemplateId) -> Option<Arc<Self>> {
        if &self.template_id != id {
            return None;
        }

        let mut instance = self.clone();
        instance.template_id = new_id.clone();
        Some(Arc::new(instance))
    }
}
