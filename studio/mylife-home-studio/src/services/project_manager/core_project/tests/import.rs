//! Taking plugins and components in from somewhere else.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use studio_web_api::project_manager::core_import_data::{ChangeType, Impact, ObjectChange};
use studio_web_api::project_manager::{
    ComponentsImportType, ImportFromOnlineConfig, ImportFromProjectConfig,
};

use super::super::import::{
    ImportData, OnlineComponent, OnlineSource, apply_updates, build_updates, from_online,
    from_project, prepare_changes,
};
use super::super::resolver::resolve;
use super::fixture::*;

// ---------------------------------------------------------------------------
// Sources
// ---------------------------------------------------------------------------

/// A registry holding the fixtures' plugin at a newer version, declaring
/// nothing, and one component of it.
///
/// The version is what the diff compares: a plugin whose members moved
/// without a new version is not reported as changed.
fn online() -> OnlineSource {
    let metadata = Arc::new(common::components::metadata::PluginMetadata::new(
        String::from("name"),
        String::from("module"),
        common::components::metadata::PluginUsage::Logic,
        String::from("2.0.0"),
        None,
        HashMap::new(),
        HashMap::new(),
    ));

    OnlineSource {
        plugins: vec![(instance_id("inst"), metadata.clone())],
        components: vec![OnlineComponent {
            id: component_id("running"),
            instance: instance_id("inst"),
            plugin: metadata,
        }],
    }
}

fn everything_from_online() -> ImportData {
    from_online(
        &online(),
        &ImportFromOnlineConfig {
            import_plugins: true,
            import_components: true,
        },
    )
    .unwrap()
}

fn from_project_with(kind: Option<ComponentsImportType>, project: &Project) -> ImportData {
    from_project(
        &resolve(project).unwrap(),
        &ImportFromProjectConfig {
            import_plugins: true,
            import_components: kind,
            project_id: String::from("other"),
        },
    )
    .unwrap()
}

fn change_keys(changes: &[ObjectChange]) -> Vec<String> {
    let mut keys: Vec<String> = changes
        .iter()
        .map(|change| change.key().to_owned())
        .collect();
    keys.sort();
    keys
}

// ---------------------------------------------------------------------------
// What a source offers
// ---------------------------------------------------------------------------

#[test]
fn a_component_taken_from_what_is_running_comes_in_external() {
    // Its configuration belongs to whoever deploys it, and is not published.
    let data = everything_from_online();

    assert_eq!(data.components.len(), 1);
    assert!(data.components[0].external);
    assert!(data.components[0].config.is_none());
}

#[test]
fn asking_for_plugins_only_brings_no_components() {
    let data = from_online(
        &online(),
        &ImportFromOnlineConfig {
            import_plugins: true,
            import_components: false,
        },
    )
    .unwrap();

    assert_eq!(data.plugins.len(), 1);
    assert!(data.components.is_empty());
}

#[test]
fn asking_for_components_brings_the_plugins_they_need() {
    let data = from_online(
        &online(),
        &ImportFromOnlineConfig {
            import_plugins: false,
            import_components: true,
        },
    )
    .unwrap();

    assert_eq!(
        data.plugins.len(),
        1,
        "a component with no plugin would be meaningless"
    );
}

#[test]
fn a_project_is_taken_in_flattened() {
    // `{{id}}-inner` inside a template used by `usage` arrives as
    // `usage-inner`: that is what will run.
    let data = from_project_with(
        Some(ComponentsImportType::Standard),
        &project_with_template(),
    );

    assert_eq!(
        data.components
            .iter()
            .map(|component| component.id.to_string())
            .collect::<Vec<_>>(),
        ["usage-inner"]
    );
}

#[test]
fn taking_a_project_in_as_external_takes_the_ids_only() {
    let standard = from_project_with(
        Some(ComponentsImportType::Standard),
        &project_with_components(),
    );
    let external = from_project_with(
        Some(ComponentsImportType::External),
        &project_with_components(),
    );

    assert!(standard.components.iter().all(|item| item.config.is_some()));
    assert!(external.components.iter().all(|item| item.config.is_none()));
    assert!(external.components.iter().all(|item| item.external));
}

// ---------------------------------------------------------------------------
// What differs
// ---------------------------------------------------------------------------

#[test]
fn a_plugin_the_project_does_not_have_is_an_addition() {
    let changes = prepare_changes(&everything_from_online(), &Project::empty()).unwrap();

    let Some(ObjectChange::Plugin(change)) = changes.first() else {
        panic!("expected a plugin change, got {:?}", change_keys(&changes));
    };

    assert!(matches!(change.change_type, ChangeType::Add));
    assert_eq!(change.id, "inst:module.name");
    assert_eq!(change.version.before, "", "there was nothing before");
    assert_eq!(change.version.after, "2.0.0");
}

#[test]
fn a_plugin_the_source_does_not_have_is_a_removal() {
    // The project knows the fixtures' plugin; an empty import does not.
    let changes = prepare_changes(&ImportData::default(), &project()).unwrap();

    assert_eq!(change_keys(&changes), ["plugin-clear:inst:module.name"]);
}

#[test]
fn a_plugin_whose_version_moved_is_an_update_naming_what_moved() {
    // The online fixture declares no members or config, and the project's
    // plugin declares several.
    let changes = prepare_changes(&everything_from_online(), &project()).unwrap();

    let Some(ObjectChange::Plugin(change)) = changes
        .iter()
        .find(|change| matches!(change, ObjectChange::Plugin(_)))
    else {
        panic!("expected a plugin change");
    };

    assert!(matches!(change.change_type, ChangeType::Update));
    assert!(matches!(
        change.members.get("state"),
        Some(ChangeType::Delete)
    ));
    assert!(matches!(
        change.config.get("label"),
        Some(ChangeType::Delete)
    ));
}

#[test]
fn a_component_the_project_already_has_unchanged_is_not_a_change() {
    let imports = from_project_with(
        Some(ComponentsImportType::Standard),
        &project_with_components(),
    );

    let changes = prepare_changes(&imports, &project_with_components()).unwrap();

    assert!(
        change_keys(&changes).is_empty(),
        "{:?}",
        change_keys(&changes)
    );
}

#[test]
fn a_component_the_source_dropped_is_removed_from_that_instance() {
    // A project deploys whole instances, so the import is the authority on
    // what runs there.
    let source = project_with_components()
        .clear_components(
            &mut Vec::new(),
            &ViewKey::Project,
            &[component_id("target")],
        )
        .unwrap();

    let imports = from_project_with(Some(ComponentsImportType::Standard), &source);
    let changes = prepare_changes(&imports, &project_with_components()).unwrap();

    assert_eq!(change_keys(&changes), ["component-clear:target"]);
}

#[test]
fn a_component_to_add_waits_for_the_plugin_it_needs() {
    let changes = prepare_changes(&everything_from_online(), &Project::empty()).unwrap();

    let Some(ObjectChange::Component(component)) = changes
        .iter()
        .find(|change| matches!(change, ObjectChange::Component(_)))
    else {
        panic!("expected a component change");
    };

    assert_eq!(component.dependencies, ["plugin-set:inst:module.name"]);
}

// ---------------------------------------------------------------------------
// What a change drags along
// ---------------------------------------------------------------------------

#[test]
fn removing_a_plugin_takes_the_components_and_their_bindings() {
    let project = project_with_binding();
    let mut changes = prepare_changes(&ImportData::default(), &project).unwrap();

    let updates = build_updates(&ImportData::default(), &project, &mut changes).unwrap();

    let mut ids: Vec<&str> = updates.iter().map(|update| update.id.as_str()).collect();
    ids.sort();

    assert_eq!(
        ids,
        [
            "binding-clear::source:state:target:action",
            "component-clear::source",
            "component-clear::target",
            "plugin-clear:inst:module.name",
        ]
    );
}

#[test]
fn the_change_says_what_it_drags_along() {
    let project = project_with_binding();
    let mut changes = prepare_changes(&ImportData::default(), &project).unwrap();

    build_updates(&ImportData::default(), &project, &mut changes).unwrap();

    let ObjectChange::Plugin(change) = &changes[0] else {
        panic!("expected a plugin change");
    };

    let deleted: Vec<&str> = change
        .impacts
        .iter()
        .filter_map(|impact| match impact {
            Impact::ComponentDelete(impact) => Some(impact.component_id.as_str()),
            _ => None,
        })
        .collect();

    assert_eq!(deleted, ["source", "target"]);
    assert!(
        change
            .impacts
            .iter()
            .any(|impact| matches!(impact, Impact::BindingDelete(_))),
        "the binding goes too"
    );
}

#[test]
fn a_config_item_a_plugin_dropped_is_cleared_on_its_components() {
    // The project's plugin has `label`; the imported one does not.
    let project = project_with_components();
    let imports = everything_from_online();

    let mut changes = prepare_changes(&imports, &project).unwrap();
    let updates = build_updates(&imports, &project, &mut changes).unwrap();

    let mut ids: Vec<&str> = updates
        .iter()
        .filter(|update| update.id.starts_with("component-clear-config"))
        .map(|update| update.id.as_str())
        .collect();
    ids.sort();

    assert!(
        ids.contains(&"component-clear-config::source:label"),
        "{ids:?}"
    );
}

#[test]
fn clearing_a_config_export_reaches_the_components_using_the_template() {
    let project = project_with_template()
        .set_template_export(
            &mut Vec::new(),
            &template_id("tpl"),
            ExportKind::Config,
            export_id("label"),
            &component_id("{{id}}-inner"),
            "label",
        )
        .unwrap();

    let mut changes = vec![ObjectChange::Template(
        studio_web_api::project_manager::core_import_data::TemplateChange {
            key: String::from("update-template"),
            id: String::from("tpl"),
            change_type: ChangeType::Update,
            dependencies: Vec::new(),
            impacts: Vec::new(),
            export_type: String::from("config"),
            export_id: String::from("label"),
        },
    )];

    let updates = build_updates(&ImportData::default(), &project, &mut changes).unwrap();

    let ids: BTreeSet<&str> = updates.iter().map(|update| update.id.as_str()).collect();

    assert!(
        ids.contains("template-clear-export:tpl:config:label"),
        "{ids:?}"
    );
    assert!(
        ids.contains("component-clear-config::usage:label"),
        "the component using the template stops supplying it: {ids:?}"
    );
    assert!(
        ids.contains("component-reset-config:tpl:{{id}}-inner:label"),
        "and the component inside it takes its own value back: {ids:?}"
    );
}

// ---------------------------------------------------------------------------
// Writing it
// ---------------------------------------------------------------------------

#[test]
fn applying_a_selection_writes_only_what_was_picked() {
    let project = project_with_binding();
    let mut changes = prepare_changes(&ImportData::default(), &project).unwrap();
    let updates = build_updates(&ImportData::default(), &project, &mut changes).unwrap();

    let mut emitted = Vec::new();
    let (applied, stats) = apply_updates(
        &project,
        &updates,
        &BTreeSet::from([String::from("plugin-clear:inst:module.name")]),
        &mut emitted,
    )
    .unwrap();

    assert!(applied.plugins().is_empty());
    assert!(component_ids(&applied, &ViewKey::Project).is_empty());
    assert!(binding_ids(&applied, &ViewKey::Project).is_empty());

    assert_eq!(stats.plugins, 1);
    assert_eq!(stats.components, 2);
    assert_eq!(stats.bindings, 1);

    assert_eq!(
        kinds(&emitted),
        [
            "clear-binding",
            "clear-component",
            "clear-component",
            "clear-plugin"
        ],
        "bindings first, the plugin last"
    );
}

#[test]
fn an_empty_selection_writes_nothing() {
    let project = project_with_binding();
    let mut changes = prepare_changes(&ImportData::default(), &project).unwrap();
    let updates = build_updates(&ImportData::default(), &project, &mut changes).unwrap();

    let mut emitted = Vec::new();
    let (applied, stats) =
        apply_updates(&project, &updates, &BTreeSet::new(), &mut emitted).unwrap();

    assert_eq!(applied.plugins().len(), 1);
    assert!(emitted.is_empty());
    assert_eq!(stats.plugins, 0);
}

#[test]
fn an_imported_component_lands_with_what_the_source_said() {
    let source = project_with_components()
        .configure_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("source"),
            "label",
            serde_json::json!("from the source"),
        )
        .unwrap();

    let imports = from_project_with(Some(ComponentsImportType::Standard), &source);
    let target = project();

    let mut changes = prepare_changes(&imports, &target).unwrap();
    let updates = build_updates(&imports, &target, &mut changes).unwrap();

    let selection: BTreeSet<String> = changes
        .iter()
        .map(|change| change.key().to_owned())
        .collect();

    let (applied, _) = apply_updates(&target, &updates, &selection, &mut Vec::new()).unwrap();

    assert_eq!(
        config_value(&applied, &ViewKey::Project, "source", "label"),
        Some(String::from("\"from the source\""))
    );
}

#[test]
fn the_writes_are_ordered_whatever_order_they_were_worked_out_in() {
    // Clearing a plugin before the components using it would be refused, so
    // applying does not trust the order the updates were built in.
    let project = project_with_binding();
    let mut changes = prepare_changes(&ImportData::default(), &project).unwrap();
    let mut updates = build_updates(&ImportData::default(), &project, &mut changes).unwrap();

    updates.reverse();

    let selection: BTreeSet<String> = changes
        .iter()
        .map(|change| change.key().to_owned())
        .collect();

    let (applied, _) = apply_updates(&project, &updates, &selection, &mut Vec::new())
        .expect("the writes still land in a workable order");

    assert!(applied.plugins().is_empty());
    assert!(component_ids(&applied, &ViewKey::Project).is_empty());
}
