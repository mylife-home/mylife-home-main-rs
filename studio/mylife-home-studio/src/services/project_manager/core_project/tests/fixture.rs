//! Builders shared by the tests.
//!
//! A fixture is built through the operations wherever an operation exists, so
//! it is a project the model could have produced. Plugins are the exception:
//! they arrive from the network rather than from an operation, so they are
//! constructed directly.

use std::collections::HashMap;
use std::sync::Arc;

pub use studio_web_api::component_model::{
    ConfigItem, ConfigType, Member, MemberType, PluginUsage,
};

/// The model's whole surface, so a test file needs one import line.
pub use crate::services::project_manager::core_project::model::*;

// ---------------------------------------------------------------------------
// Ids
// ---------------------------------------------------------------------------

pub fn component_id(id: &str) -> ComponentId {
    ComponentId::new(id).unwrap()
}

pub fn template_id(id: &str) -> TemplateId {
    TemplateId::new(id).unwrap()
}

pub fn export_id(id: &str) -> ExportId {
    ExportId::new(id).unwrap()
}

pub fn instance_id(id: &str) -> InstanceId {
    InstanceId::new(id).unwrap()
}

pub fn template_view(id: &str) -> ViewKey {
    ViewKey::Template(template_id(id))
}

// ---------------------------------------------------------------------------
// Plugins
// ---------------------------------------------------------------------------

pub fn member(member_type: MemberType, value_type: &str) -> Member {
    Member {
        description: None,
        member_type,
        value_type: String::from(value_type),
    }
}

pub fn config_item(value_type: ConfigType) -> ConfigItem {
    ConfigItem {
        description: None,
        value_type,
    }
}

/// A plugin with a `bool` state, a `bool` action, a range state, and two
/// config items of different types: enough to exercise every check the model
/// makes about members and configuration.
pub fn plugin_of(instance: &InstanceId, name: &str) -> Plugin {
    Plugin {
        name: String::from(name),
        module: String::from("module"),
        usage: PluginUsage::Logic,
        version: String::from("1.0.0"),
        description: None,
        members: HashMap::from([
            (String::from("state"), member(MemberType::State, "bool")),
            (String::from("action"), member(MemberType::Action, "bool")),
            (
                String::from("level"),
                member(MemberType::State, "range[0;100]"),
            ),
        ]),
        config: HashMap::from([
            (String::from("label"), config_item(ConfigType::String)),
            (String::from("count"), config_item(ConfigType::Integer)),
        ]),
        instance_name: instance.clone(),
        toolbox_display: ToolboxDisplay::Show,
    }
}

pub fn plugin_id() -> PluginId {
    PluginId::build(&instance_id("inst"), "module", "name").unwrap()
}

pub fn plugin_definition() -> ComponentDefinition {
    ComponentDefinition::Plugin(plugin_id())
}

pub fn template_definition(id: &str) -> ComponentDefinition {
    ComponentDefinition::Template(template_id(id))
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

/// An empty project that knows one plugin, `inst:module.name`.
pub fn project() -> Project {
    let instance = instance_id("inst");

    Project::from_parts(
        Collection::new(),
        Collection::new(),
        Collection::from_iter([(plugin_id(), Arc::new(plugin_of(&instance, "name")))]),
        Collection::new(),
    )
}

/// `project()` plus a second plugin on a second instance, so the
/// instance-level operations have something to tell apart.
pub fn project_with_two_instances() -> Project {
    let other = instance_id("other");
    let other_plugin = PluginId::build(&other, "module", "name").unwrap();
    let second = PluginId::build(&instance_id("inst"), "module", "second").unwrap();

    let instance = instance_id("inst");

    Project::from_parts(
        Collection::new(),
        Collection::new(),
        Collection::from_iter([
            (plugin_id(), Arc::new(plugin_of(&instance, "name"))),
            (second, Arc::new(plugin_of(&instance, "second"))),
            (other_plugin, Arc::new(plugin_of(&other, "name"))),
        ]),
        Collection::new(),
    )
}

/// `project()` with two components of that plugin on the project view.
pub fn project_with_components() -> Project {
    project()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("source"),
            plugin_definition(),
            Position { x: 0, y: 0 },
        )
        .unwrap()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("target"),
            plugin_definition(),
            Position { x: 10, y: 10 },
        )
        .unwrap()
}

/// `project_with_components()` with `source.state` bound to `target.action`.
pub fn project_with_binding() -> Project {
    project_with_components()
        .set_binding(
            &mut Vec::new(),
            &ViewKey::Project,
            binding("source", "state", "target", "action"),
        )
        .unwrap()
}

/// A template `tpl` holding one component, used once on the project view by a
/// component called `usage`.
///
/// The inner component carries the `{{id}}` placeholder, without which the
/// template could only ever be used once.
pub fn project_with_template() -> Project {
    project()
        .set_template(&mut Vec::new(), template_id("tpl"))
        .unwrap()
        .set_component(
            &mut Vec::new(),
            &template_view("tpl"),
            component_id("{{id}}-inner"),
            plugin_definition(),
            Position::default(),
        )
        .unwrap()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("usage"),
            template_definition("tpl"),
            Position::default(),
        )
        .unwrap()
}

/// A project with one of most things in it: a hidden plugin, a template with
/// both kinds of export, the component that uses it, and a bound pair.
///
/// What the round-trip and initial-state tests need.
pub fn populated_project() -> Project {
    project_with_template()
        .set_template_export(
            &mut Vec::new(),
            &template_id("tpl"),
            ExportKind::Config,
            export_id("label"),
            &component_id("{{id}}-inner"),
            "label",
        )
        .unwrap()
        .set_template_export(
            &mut Vec::new(),
            &template_id("tpl"),
            ExportKind::Member,
            export_id("out"),
            &component_id("{{id}}-inner"),
            "state",
        )
        .unwrap()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("source"),
            plugin_definition(),
            Position { x: 1, y: 2 },
        )
        .unwrap()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("target"),
            plugin_definition(),
            Position { x: 3, y: 4 },
        )
        .unwrap()
        .set_binding(
            &mut Vec::new(),
            &ViewKey::Project,
            binding("source", "state", "target", "action"),
        )
        .unwrap()
        .set_plugin_display(&mut Vec::new(), &plugin_id(), ToolboxDisplay::Hide)
        .unwrap()
        .expect("the display changed")
}

// ---------------------------------------------------------------------------
// Elements
// ---------------------------------------------------------------------------

pub fn binding(source: &str, state: &str, target: &str, action: &str) -> Binding {
    Binding {
        source_component: component_id(source),
        source_state: String::from(state),
        target_component: component_id(target),
        target_action: String::from(action),
    }
}

// ---------------------------------------------------------------------------
// Reading a project back
// ---------------------------------------------------------------------------

pub fn component_of(project: &Project, view: &ViewKey, id: &str) -> Arc<Component> {
    project
        .view(view)
        .unwrap()
        .components
        .get_required(&component_id(id))
        .unwrap()
        .clone()
}

pub fn component_ids(project: &Project, view: &ViewKey) -> Vec<String> {
    let mut ids: Vec<String> = project
        .view(view)
        .unwrap()
        .components
        .keys()
        .map(ToString::to_string)
        .collect();
    ids.sort();
    ids
}

pub fn binding_ids(project: &Project, view: &ViewKey) -> Vec<String> {
    let mut ids: Vec<String> = project
        .view(view)
        .unwrap()
        .bindings
        .keys()
        .map(ToString::to_string)
        .collect();
    ids.sort();
    ids
}

pub fn config_value(project: &Project, view: &ViewKey, id: &str, name: &str) -> Option<String> {
    component_of(project, view, id)
        .config
        .as_ref()?
        .get(name)
        .map(ToString::to_string)
}

// ---------------------------------------------------------------------------
// Changes
// ---------------------------------------------------------------------------

/// One short name per change, so a test can say what was emitted without
/// matching on every field of it.
pub fn kind(change: &Change) -> &'static str {
    match change {
        Change::Reset => "reset",
        Change::SetPlugins(_) => "set-plugins",
        Change::SetPluginToolboxDisplay { .. } => "set-plugin-display",
        Change::SetPlugin { .. } => "set-plugin",
        Change::ClearPlugin { .. } => "clear-plugin",
        Change::SetTemplate { .. } => "set-template",
        Change::ClearTemplate { .. } => "clear-template",
        Change::RenameTemplate { .. } => "rename-template",
        Change::SetComponent { .. } => "set-component",
        Change::ClearComponent { .. } => "clear-component",
        Change::RenameComponent { .. } => "rename-component",
        Change::SetBinding { .. } => "set-binding",
        Change::ClearBinding { .. } => "clear-binding",
    }
}

pub fn kinds(changes: &[Change]) -> Vec<&'static str> {
    changes.iter().map(kind).collect()
}

/// The component ids a batch of changes names, in order, so a test can assert
/// on what was announced rather than only on what the project ended up with.
pub fn changed_components(changes: &[Change]) -> Vec<String> {
    changes
        .iter()
        .filter_map(|change| match change {
            Change::SetComponent { id, .. } | Change::ClearComponent { id, .. } => {
                Some(id.to_string())
            }
            Change::RenameComponent { id, new_id, .. } => Some(format!("{id}->{new_id}")),
            _ => None,
        })
        .collect()
}

pub fn changed_bindings(changes: &[Change]) -> Vec<String> {
    changes
        .iter()
        .filter_map(|change| match change {
            Change::SetBinding { id, .. } => Some(format!("set {id}")),
            Change::ClearBinding { id, .. } => Some(format!("clear {id}")),
            _ => None,
        })
        .collect()
}
