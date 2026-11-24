//! Builders shared by the tests.
//!
//! A fixture is built through the operations wherever an operation exists, so
//! it is a project the model could have produced. Component and plugin data is
//! the exception: it arrives from the core side rather than from an operation,
//! so it is constructed directly.

use std::collections::HashMap;
use std::sync::Arc;

pub use studio_web_api::component_model::{Member, MemberType};

/// The model's whole surface, so a test file needs one import line.
pub use crate::services::project_manager::ui_project::model::*;

// ---------------------------------------------------------------------------
// Ids
//
// `WindowId::new("win").unwrap()` in every test would bury what each test is
// actually about.
// ---------------------------------------------------------------------------

pub fn resource_id(id: &str) -> ResourceId {
    ResourceId::new(id).unwrap()
}

pub fn style_id(id: &str) -> StyleId {
    StyleId::new(id).unwrap()
}

pub fn window_id(id: &str) -> WindowId {
    WindowId::new(id).unwrap()
}

pub fn template_id(id: &str) -> TemplateId {
    TemplateId::new(id).unwrap()
}

pub fn control_id(id: &str) -> ControlId {
    ControlId::new(id).unwrap()
}

pub fn instance_id(id: &str) -> TemplateInstanceId {
    TemplateInstanceId::new(id).unwrap()
}

pub fn export_id(id: &str) -> ExportId {
    ExportId::new(id).unwrap()
}

pub fn component_id(id: &str) -> ComponentId {
    ComponentId::new(id).unwrap()
}

pub fn window_view(id: &str) -> ViewKey {
    ViewKey::Window(window_id(id))
}

pub fn template_view(id: &str) -> ViewKey {
    ViewKey::Template(template_id(id))
}

// ---------------------------------------------------------------------------
// Component data
// ---------------------------------------------------------------------------

pub fn member(member_type: MemberType, value_type: &str) -> Member {
    Member {
        description: None,
        member_type,
        value_type: String::from(value_type),
    }
}

pub fn plugin_id() -> PluginId {
    PluginId::build("instance", "module", "name").unwrap()
}

/// The plugin every fixture component uses: `state` and `action` are bools,
/// `level` is a range and `label` is text, which is enough to exercise every
/// type check the model and the deploy pass make.
pub fn plugin() -> Plugin {
    Plugin {
        name: String::from("name"),
        module: String::from("module"),
        version: String::from("1.0.0"),
        description: None,
        members: HashMap::from([
            (String::from("state"), member(MemberType::State, "bool")),
            (String::from("action"), member(MemberType::Action, "bool")),
            (
                String::from("level"),
                member(MemberType::State, "range[0;100]"),
            ),
            (String::from("label"), member(MemberType::State, "text")),
        ]),
        instance_name: String::from("instance"),
    }
}

/// Component data as it arrives from the core side: one component, `comp`.
pub fn component_data() -> (
    Collection<ComponentId, Component>,
    Collection<PluginId, Plugin>,
) {
    let plugin_id = plugin_id();

    let components = Collection::from_iter([(
        component_id("comp"),
        Arc::new(Component {
            plugin: plugin_id.clone(),
        }),
    )]);
    let plugins = Collection::from_iter([(plugin_id, Arc::new(plugin()))]);

    (components, plugins)
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

/// An empty project that knows about component `comp`.
pub fn project() -> Project {
    let (components, plugins) = component_data();

    Project::from_parts(
        Collection::new(),
        Collection::new(),
        Collection::new(),
        Collection::new(),
        Arc::new(DefaultWindow::default()),
        components,
        plugins,
    )
}

/// `project()` plus an empty window, `win`.
pub fn project_with_window() -> Project {
    project()
        .new_window(&mut Vec::new(), window_id("win"))
        .unwrap()
}

/// `project()` plus a template `tpl` exporting `exp`, a bool state.
pub fn project_with_template() -> Project {
    project()
        .new_template(&mut Vec::new(), template_id("tpl"))
        .unwrap()
        .set_template_export(
            &mut Vec::new(),
            template_id("tpl"),
            export_id("exp"),
            MemberType::State,
            "bool",
        )
        .unwrap()
}

/// A window holding a display control, and a template holding one instance of
/// `tpl`. The shape most of the reference-following tests need.
pub fn project_with_view_contents() -> Project {
    project_with_template()
        .new_window(&mut Vec::new(), window_id("win"))
        .unwrap()
        .new_control(
            &mut Vec::new(),
            &window_view("win"),
            control_id("ctl"),
            10,
            20,
            ControlKind::Display,
        )
        .unwrap()
        .new_template_instance(
            &mut Vec::new(),
            &window_view("win"),
            instance_id("inst"),
            template_id("tpl"),
            30,
            40,
        )
        .unwrap()
}

/// `project_with_view_contents()` with one of most things set: a style, a
/// resource, a default-window map, a window using both, a bound display and a
/// bound instance.
///
/// It is deployable, which is what the deploy tests start from: every
/// reference in it points at something, so any error a test sees is the one it
/// introduced.
pub fn populated_project() -> Project {
    let project = project_with_view_contents()
        .set_style(
            &mut Vec::new(),
            style_id("big"),
            StyleDefinition {
                properties: serde_json::json!({ "font-size": "2em" }),
            },
        )
        .set_resource(
            &mut Vec::new(),
            resource_id("bg"),
            Resource {
                mime: String::from("image/png"),
                data: String::from("AAAA"),
            },
        )
        .set_default_window(
            &mut Vec::new(),
            DefaultWindow::new(HashMap::from([(
                String::from("desktop"),
                Some(window_id("win")),
            )])),
        );

    let project = project
        .set_window_properties(
            &mut Vec::new(),
            window_id("win"),
            WindowProperties {
                style: Patch::Set(StyleList::new(vec![style_id("big")])),
                background_resource: Patch::Set(Some(resource_id("bg"))),
                ..Default::default()
            },
        )
        .unwrap()
        .expect("the window changed");

    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        display_properties(Some(component_ref("comp", "state"))),
    );

    project
        .set_template_instance_bindings(
            &mut Vec::new(),
            &window_view("win"),
            instance_id("inst"),
            HashMap::from([(export_id("exp"), Some(component_ref("comp", "state")))]),
        )
        .unwrap()
}

// ---------------------------------------------------------------------------
// Elements
// ---------------------------------------------------------------------------

pub fn display_with(member: Option<MemberRef>) -> Arc<ControlDisplay> {
    Arc::new(ControlDisplay {
        member,
        default_resource: None,
        map: Vec::new(),
    })
}

pub fn component_action(member: Option<MemberRef>) -> Arc<Action> {
    Arc::new(Action {
        component: Some(ActionComponent { member }),
        window: None,
    })
}

pub fn window_action(id: Option<WindowId>) -> Arc<Action> {
    Arc::new(Action {
        component: None,
        window: Some(ActionWindow { id, popup: false }),
    })
}

pub fn text_with(context: Vec<ControlTextContextItem>) -> Arc<ControlText> {
    Arc::new(ControlText {
        context,
        format: String::from("return '';"),
    })
}

pub fn context_item(id: &str, member: Option<MemberRef>) -> ControlTextContextItem {
    ControlTextContextItem {
        id: String::from(id),
        member,
        test_value: serde_json::Value::Null,
    }
}

pub fn component_ref(component: &str, member: &str) -> MemberRef {
    MemberRef::component(component_id(component), member)
}

/// Replace a control's parts in one call, since every reference test starts by
/// putting a reference somewhere.
pub fn set_control(
    project: &Project,
    view: &ViewKey,
    id: &ControlId,
    properties: ControlProperties,
) -> Project {
    project
        .set_control_properties(&mut Vec::new(), view, id.clone(), properties)
        .unwrap()
        .expect("the control changed")
}

pub fn display_properties(member: Option<MemberRef>) -> ControlProperties {
    ControlProperties {
        display: Patch::Set(Some(display_with(member))),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Changes
// ---------------------------------------------------------------------------

/// One short name per change, so a test can say what was emitted without
/// matching on every field of it.
pub fn kind(change: &Change) -> &'static str {
    match change {
        Change::Reset => "reset",
        Change::SetDefaultWindow(_) => "set-default-window",
        Change::SetResource { .. } => "set-resource",
        Change::ClearResource { .. } => "clear-resource",
        Change::RenameResource { .. } => "rename-resource",
        Change::SetStyle { .. } => "set-style",
        Change::ClearStyle { .. } => "clear-style",
        Change::RenameStyle { .. } => "rename-style",
        Change::SetWindow { .. } => "set-window",
        Change::ClearWindow { .. } => "clear-window",
        Change::RenameWindow { .. } => "rename-window",
        Change::SetTemplate { .. } => "set-template",
        Change::ClearTemplate { .. } => "clear-template",
        Change::RenameTemplate { .. } => "rename-template",
        Change::SetComponentData { .. } => "set-component-data",
    }
}

pub fn kinds(changes: &[Change]) -> Vec<&'static str> {
    changes.iter().map(kind).collect()
}
