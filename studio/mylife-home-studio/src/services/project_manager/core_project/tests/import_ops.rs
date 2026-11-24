//! The writes a bulk update makes.
//!
//! These are the operations an accepted import applies. The checks live in the
//! update builder, which worked the whole cascade out before the user saw it,
//! so these write what they are told.

use std::collections::HashMap;

use super::fixture::*;

fn imported_plugin() -> Plugin {
    let mut plugin = plugin_of(&instance_id("inst"), "name");
    plugin.version = String::from("2.0.0");
    plugin
}

#[test]
fn importing_a_plugin_that_is_new_adds_it() {
    let mut changes = Vec::new();
    let project = project().import_plugin(
        &mut changes,
        PluginId::new("other:module.name").unwrap(),
        imported_plugin(),
    );

    assert_eq!(project.plugins().len(), 2);
    assert_eq!(kinds(&changes), ["set-plugin"]);
}

#[test]
fn importing_a_plugin_that_exists_replaces_it() {
    let mut changes = Vec::new();
    let project = project().import_plugin(&mut changes, plugin_id(), imported_plugin());

    assert_eq!(project.plugins().len(), 1, "replaced, not added");
    assert_eq!(
        project
            .plugins()
            .get_required(&plugin_id())
            .unwrap()
            .version,
        "2.0.0"
    );
    assert_eq!(kinds(&changes), ["set-plugin"]);
}

#[test]
fn importing_a_component_that_is_new_places_it() {
    let mut changes = Vec::new();
    let project = project()
        .import_component(
            &mut changes,
            component_id("imported"),
            plugin_id(),
            true,
            None,
        )
        .unwrap();

    let component = component_of(&project, &ViewKey::Project, "imported");

    assert!(component.external);
    assert!(component.config.is_none(), "an external component has none");
    assert_eq!(component.position, Position { x: 1, y: 1 });
    assert_eq!(kinds(&changes), ["set-component"]);
}

#[test]
fn importing_a_component_that_exists_keeps_where_it_was_put() {
    // An import describes what runs, not how the designer arranged it.
    let project = project_with_components()
        .move_components(
            &mut Vec::new(),
            &ViewKey::Project,
            &[component_id("source")],
            Position { x: 40, y: 50 },
        )
        .unwrap();

    let project = project
        .import_component(
            &mut Vec::new(),
            component_id("source"),
            plugin_id(),
            false,
            Some(Config::new(HashMap::from([(
                String::from("label"),
                serde_json::json!("imported"),
            )]))),
        )
        .unwrap();

    let component = component_of(&project, &ViewKey::Project, "source");

    assert_eq!(component.position, Position { x: 40, y: 50 });
    assert_eq!(
        config_value(&project, &ViewKey::Project, "source", "label"),
        Some(String::from("\"imported\""))
    );
}

#[test]
fn resetting_a_config_item_puts_the_default_back() {
    let project = project_with_components()
        .configure_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("source"),
            "label",
            serde_json::json!("set by hand"),
        )
        .unwrap();

    let mut changes = Vec::new();
    let project = project
        .import_reset_component_config(
            &mut changes,
            &ViewKey::Project,
            component_id("source"),
            "label",
        )
        .unwrap();

    assert_eq!(
        config_value(&project, &ViewKey::Project, "source", "label"),
        Some(String::from("\"\""))
    );
    assert_eq!(kinds(&changes), ["set-component"]);
}

#[test]
fn resetting_a_config_item_reaches_inside_a_template() {
    let project = project_with_template()
        .import_reset_component_config(
            &mut Vec::new(),
            &template_view("tpl"),
            component_id("{{id}}-inner"),
            "count",
        )
        .unwrap();

    assert_eq!(
        config_value(&project, &template_view("tpl"), "{{id}}-inner", "count"),
        Some(String::from("0"))
    );
}

#[test]
fn clearing_a_config_item_removes_the_key() {
    // What happens when a plugin stops declaring it.
    let mut changes = Vec::new();
    let project = project_with_components()
        .import_clear_component_config(
            &mut changes,
            &ViewKey::Project,
            component_id("source"),
            "label",
        )
        .unwrap();

    assert_eq!(
        config_value(&project, &ViewKey::Project, "source", "label"),
        None
    );
    assert!(
        config_value(&project, &ViewKey::Project, "source", "count").is_some(),
        "the others are untouched"
    );
    assert_eq!(kinds(&changes), ["set-component"]);
}

#[test]
fn clearing_a_config_export_drops_it_from_the_template() {
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

    let mut changes = Vec::new();
    let project = project
        .import_clear_template_export(
            &mut changes,
            &template_id("tpl"),
            ExportKind::Config,
            &export_id("label"),
        )
        .unwrap();

    let template = project
        .templates()
        .get_required(&template_id("tpl"))
        .unwrap();

    assert!(template.exports.config.is_empty());
    assert_eq!(kinds(&changes), ["set-template"]);
}

#[test]
fn clearing_a_member_export_leaves_the_config_exports_alone() {
    let project = project_with_template()
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
        .unwrap();

    let project = project
        .import_clear_template_export(
            &mut Vec::new(),
            &template_id("tpl"),
            ExportKind::Member,
            &export_id("out"),
        )
        .unwrap();

    let template = project
        .templates()
        .get_required(&template_id("tpl"))
        .unwrap();

    assert!(template.exports.members.is_empty());
    assert_eq!(template.exports.config.len(), 1);
}

#[test]
fn clearing_an_export_of_a_template_that_is_not_there_is_refused() {
    assert!(matches!(
        project().import_clear_template_export(
            &mut Vec::new(),
            &template_id("nope"),
            ExportKind::Config,
            &export_id("x"),
        ),
        Err(CoreModelError::NotFound {
            kind: IdKind::Template,
            ..
        })
    ));
}
