//! Checking a project against what is running.

use std::collections::HashMap;
use std::sync::Arc;

use studio_web_api::project_manager::core_validation::{ChangeType, Item, Severity};

use super::super::resolver::resolve;
use super::super::validation::{Online, OnlineComponent, Options, validate};
use super::fixture::*;

// ---------------------------------------------------------------------------
// What is running
// ---------------------------------------------------------------------------

/// The fixtures' plugin, as the registry reports it.
fn running_plugin() -> Arc<Plugin> {
    Arc::new(plugin_of(&instance_id("inst"), "name"))
}

/// The same, with its members or config changed.
fn running_plugin_with(
    members: HashMap<String, Member>,
    config: HashMap<String, ConfigItem>,
    version: &str,
) -> Arc<Plugin> {
    let mut plugin = plugin_of(&instance_id("inst"), "name");
    plugin.members = members;
    plugin.config = config;
    plugin.version = String::from(version);
    Arc::new(plugin)
}

fn standard_config() -> HashMap<String, ConfigItem> {
    HashMap::from([
        (String::from("label"), config_item(ConfigType::String)),
        (String::from("count"), config_item(ConfigType::Integer)),
    ])
}

fn standard_members() -> HashMap<String, Member> {
    HashMap::from([
        (String::from("state"), member(MemberType::State, "bool")),
        (String::from("action"), member(MemberType::Action, "bool")),
        (
            String::from("level"),
            member(MemberType::State, "range[0;100]"),
        ),
    ])
}

/// Nothing running at all.
fn nothing_online() -> Online {
    Online::new(Vec::new(), Vec::new(), Vec::new())
}

/// The fixtures' plugin is running on its own instance, and one instance can
/// carry bindings.
fn everything_matches() -> Online {
    Online::new(
        [(plugin_id(), running_plugin())],
        Vec::new(),
        vec![String::from("binder")],
    )
}

fn checked(project: &Project, online: &Online) -> Vec<Item> {
    let resolved = resolve(project).expect("the project resolves");

    validate(
        &resolved,
        online,
        Options {
            online_severity: Severity::Error,
            check_bindings_instance: true,
        },
    )
    .expect("the check runs")
}

fn kinds(items: &[Item]) -> Vec<&'static str> {
    items
        .iter()
        .map(|item| match item {
            Item::PluginChanged(_) => "plugin-changed",
            Item::ExistingComponentId(_) => "existing-component-id",
            Item::BadExternalComponent(_) => "bad-external-component",
            Item::InvalidBindingApi(_) => "invalid-binding-api",
            Item::ComponentBadConfig(_) => "component-bad-config",
            Item::BindingMismatch(_) => "binding-mismatch",
        })
        .collect()
}

// ---------------------------------------------------------------------------

#[test]
fn a_project_that_matches_what_is_running_has_nothing_to_say() {
    assert!(checked(&project_with_binding(), &everything_matches()).is_empty());
}

#[test]
fn a_plugin_that_is_gone_is_reported_with_what_it_breaks() {
    let items = checked(&project_with_binding(), &nothing_online());

    let Some(Item::PluginChanged(item)) = items
        .iter()
        .find(|item| matches!(item, Item::PluginChanged(_)))
    else {
        panic!("expected a plugin change, got {:?}", kinds(&items));
    };

    assert!(matches!(item.change_type, ChangeType::Delete));
    assert_eq!(item.instance_name, "inst");
    assert_eq!(item.module, "module");
    assert_eq!(item.name, "name");
    assert_eq!(
        item.impacts,
        ["source", "target"],
        "every component that instantiates it"
    );
}

#[test]
fn a_member_that_changed_type_is_named() {
    let online = Online::new(
        [(
            plugin_id(),
            running_plugin_with(
                HashMap::from([
                    // was bool
                    (String::from("state"), member(MemberType::State, "float")),
                    (String::from("action"), member(MemberType::Action, "bool")),
                ]),
                standard_config(),
                "1.0.0",
            ),
        )],
        Vec::new(),
        vec![String::from("binder")],
    );

    let items = checked(&project_with_binding(), &online);

    let Some(Item::PluginChanged(item)) = items
        .iter()
        .find(|item| matches!(item, Item::PluginChanged(_)))
    else {
        panic!("expected a plugin change, got {:?}", kinds(&items));
    };

    assert!(matches!(item.change_type, ChangeType::Update));
    assert!(matches!(
        item.members.get("state"),
        Some(ChangeType::Update)
    ));
    assert!(
        matches!(item.members.get("level"), Some(ChangeType::Delete)),
        "the project has a member the running plugin does not"
    );
    assert!(item.config.is_empty(), "the config is unchanged");
}

#[test]
fn a_config_item_the_running_plugin_gained_is_an_addition() {
    let online = Online::new(
        [(
            plugin_id(),
            running_plugin_with(
                standard_members(),
                HashMap::from([
                    (String::from("label"), config_item(ConfigType::String)),
                    (String::from("count"), config_item(ConfigType::Integer)),
                    (String::from("extra"), config_item(ConfigType::Bool)),
                ]),
                "1.0.0",
            ),
        )],
        Vec::new(),
        vec![String::from("binder")],
    );

    let items = checked(&project_with_binding(), &online);

    let Some(Item::PluginChanged(item)) = items
        .iter()
        .find(|item| matches!(item, Item::PluginChanged(_)))
    else {
        panic!("expected a plugin change, got {:?}", kinds(&items));
    };

    assert!(matches!(item.config.get("extra"), Some(ChangeType::Add)));
    assert!(item.members.is_empty());
}

#[test]
fn a_component_id_already_taken_on_another_instance_is_reported() {
    let online = Online::new(
        [(plugin_id(), running_plugin())],
        [(
            component_id("source"),
            OnlineComponent {
                instance: instance_id("elsewhere"),
                plugin: running_plugin(),
            },
        )],
        vec![String::from("binder")],
    );

    let items = checked(&project_with_binding(), &online);

    let Some(Item::ExistingComponentId(item)) = items
        .iter()
        .find(|item| matches!(item, Item::ExistingComponentId(_)))
    else {
        panic!("expected an id clash, got {:?}", kinds(&items));
    };

    assert_eq!(item.component_id, "source");
    assert_eq!(item.existing.instance_name, "elsewhere");
    assert_eq!(item.project.instance_name, "inst");
}

#[test]
fn the_same_id_on_the_instance_it_belongs_to_is_fine() {
    // That is just this project's own component, already deployed.
    let online = Online::new(
        [(plugin_id(), running_plugin())],
        [(
            component_id("source"),
            OnlineComponent {
                instance: instance_id("inst"),
                plugin: running_plugin(),
            },
        )],
        vec![String::from("binder")],
    );

    assert!(checked(&project_with_binding(), &online).is_empty());
}

// ---------------------------------------------------------------------------
// Bindings
// ---------------------------------------------------------------------------

#[test]
fn bindings_need_exactly_one_instance_to_carry_them() {
    let online = Online::new([(plugin_id(), running_plugin())], Vec::new(), Vec::new());

    let items = checked(&project_with_binding(), &online);

    let Some(Item::InvalidBindingApi(item)) = items
        .iter()
        .find(|item| matches!(item, Item::InvalidBindingApi(_)))
    else {
        panic!("expected a binding api problem, got {:?}", kinds(&items));
    };

    assert!(item.instance_names.is_empty());
    assert!(matches!(item.severity, Severity::Error));
}

#[test]
fn two_instances_offering_to_carry_bindings_is_also_wrong() {
    let online = Online::new(
        [(plugin_id(), running_plugin())],
        Vec::new(),
        vec![String::from("one"), String::from("two")],
    );

    assert_eq!(
        kinds(&checked(&project_with_binding(), &online)),
        ["invalid-binding-api"]
    );
}

#[test]
fn a_project_with_no_bindings_does_not_need_one() {
    let online = Online::new([(plugin_id(), running_plugin())], Vec::new(), Vec::new());

    assert!(checked(&project_with_components(), &online).is_empty());
}

#[test]
fn the_check_that_only_a_deploy_needs_can_be_left_out() {
    let resolved = resolve(&project_with_binding()).unwrap();
    let online = Online::new([(plugin_id(), running_plugin())], Vec::new(), Vec::new());

    let items = validate(
        &resolved,
        &online,
        Options {
            online_severity: Severity::Warning,
            check_bindings_instance: false,
        },
    )
    .unwrap();

    assert!(items.is_empty());
}

#[test]
fn a_binding_whose_ends_carry_different_types_is_reported() {
    // Reachable only from a project file written by hand: `set_binding`
    // refuses it.
    let project = project_with_components()
        .set_binding(
            &mut Vec::new(),
            &ViewKey::Project,
            binding("source", "state", "target", "action"),
        )
        .unwrap();

    let online = Online::new(
        [(plugin_id(), running_plugin())],
        Vec::new(),
        vec![String::from("binder")],
    );

    // The project's own plugin is what the types are read from, so changing
    // it is what makes the binding inconsistent.
    let mut items = checked(&project, &online);
    assert!(items.is_empty());

    let project = broken_binding_project();
    items = checked(&project, &online);

    let Some(Item::BindingMismatch(item)) = items
        .iter()
        .find(|item| matches!(item, Item::BindingMismatch(_)))
    else {
        panic!("expected a binding mismatch, got {:?}", kinds(&items));
    };

    assert_eq!(item.source_component, "source");
    assert_eq!(item.source_state, "nope");
    assert_eq!(item.source_type, None);
}

/// A binding naming a member the plugin does not have, which no operation can
/// produce.
fn broken_binding_project() -> Project {
    let project = project_with_components();
    let view = project.view(&ViewKey::Project).unwrap();

    let broken = binding("source", "nope", "target", "action");
    let bindings = view
        .bindings
        .inserted(broken.id().unwrap(), Arc::new(broken))
        .unwrap();

    project.with_view_set(view.with_bindings(bindings)).unwrap()
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

#[test]
fn a_config_key_the_plugin_does_not_declare_is_reported() {
    let project = project_with_components();
    let view = project.view(&ViewKey::Project).unwrap();

    let component = view
        .components
        .get_required(&component_id("source"))
        .unwrap();
    let component = component.with_config_value_unchecked("stray", serde_json::json!("x"));
    let view = view
        .with_component_updated(component_id("source"), component)
        .unwrap();
    let project = project.with_view_set(view).unwrap();

    let items = checked(&project, &everything_matches());

    let Some(Item::ComponentBadConfig(item)) = items
        .iter()
        .find(|item| matches!(item, Item::ComponentBadConfig(_)))
    else {
        panic!("expected a bad config, got {:?}", kinds(&items));
    };

    assert_eq!(item.component_id, "source");
    assert_eq!(
        item.config.get("stray").map(String::as_str),
        Some("Extra config key")
    );
}

#[test]
fn a_config_value_of_the_wrong_type_is_reported() {
    let project = project_with_components();
    let view = project.view(&ViewKey::Project).unwrap();

    let component = view
        .components
        .get_required(&component_id("source"))
        .unwrap();
    let component =
        component.with_config_value_unchecked("count", serde_json::json!("not a number"));
    let view = view
        .with_component_updated(component_id("source"), component)
        .unwrap();
    let project = project.with_view_set(view).unwrap();

    let items = checked(&project, &everything_matches());

    let Some(Item::ComponentBadConfig(item)) = items
        .iter()
        .find(|item| matches!(item, Item::ComponentBadConfig(_)))
    else {
        panic!("expected a bad config, got {:?}", kinds(&items));
    };

    assert!(item.config.contains_key("count"));
}

#[test]
fn an_error_anywhere_is_what_stops_a_deploy() {
    let items = checked(&broken_binding_project(), &everything_matches());

    assert!(Item::has_error(&items));
    assert!(!Item::has_error(&[]));
}
