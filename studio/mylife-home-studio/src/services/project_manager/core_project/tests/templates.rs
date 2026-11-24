//! Templates and their exports: creating, deleting, renaming, and handing a
//! member or a config item out to whoever instantiates the template.

use super::fixture::*;

fn exports(project: &Project, id: &str) -> TemplateExports {
    project
        .templates()
        .get_required(&template_id(id))
        .unwrap()
        .exports
        .clone()
}

fn export_a_config(project: &Project) -> (Project, Vec<Change>) {
    let mut changes = Vec::new();
    let project = project
        .set_template_export(
            &mut changes,
            &template_id("tpl"),
            ExportKind::Config,
            export_id("label"),
            &component_id("{{id}}-inner"),
            "label",
        )
        .unwrap();

    (project, changes)
}

// ---------------------------------------------------------------------------
// The template itself
// ---------------------------------------------------------------------------

#[test]
fn a_new_template_starts_empty_and_is_announced() {
    let mut changes = Vec::new();
    let project = project()
        .set_template(&mut changes, template_id("tpl"))
        .unwrap();

    let template = project
        .templates()
        .get_required(&template_id("tpl"))
        .unwrap();

    assert!(template.components.is_empty());
    assert!(template.exports.config.is_empty());
    assert!(template.exports.members.is_empty());
    assert_eq!(kinds(&changes), ["set-template"]);
}

#[test]
fn a_new_template_cannot_take_an_id_already_in_use() {
    let error = project_with_template()
        .set_template(&mut Vec::new(), template_id("tpl"))
        .unwrap_err();

    assert!(matches!(
        error,
        CoreModelError::AlreadyExists {
            kind: IdKind::Template,
            ..
        }
    ));
}

#[test]
fn deleting_a_template_takes_its_contents_with_it() {
    // The template's own components and bindings go on one change: the client
    // drops the whole view.
    let project = project()
        .set_template(&mut Vec::new(), template_id("tpl"))
        .unwrap()
        .set_component(
            &mut Vec::new(),
            &template_view("tpl"),
            component_id("inner"),
            plugin_definition(),
            Position::default(),
        )
        .unwrap();

    let mut changes = Vec::new();
    let project = project
        .clear_template(&mut changes, &template_id("tpl"))
        .unwrap()
        .expect("it was there");

    assert!(project.templates().is_empty());
    assert_eq!(kinds(&changes), ["clear-template"]);
}

#[test]
fn a_template_a_component_instantiates_cannot_be_deleted() {
    let mut changes = Vec::new();
    let error = project_with_template()
        .clear_template(&mut changes, &template_id("tpl"))
        .unwrap_err();

    assert!(matches!(
        error,
        CoreModelError::InUse {
            kind: IdKind::Template,
            ..
        }
    ));
    assert!(changes.is_empty());
}

#[test]
fn deleting_an_unknown_template_is_refused() {
    assert!(matches!(
        project().clear_template(&mut Vec::new(), &template_id("nope")),
        Err(CoreModelError::NotFound {
            kind: IdKind::Template,
            ..
        })
    ));
}

#[test]
fn renaming_a_template_repoints_the_components_using_it() {
    let mut changes = Vec::new();
    let project = project_with_template()
        .rename_template(&mut changes, &template_id("tpl"), template_id("renamed"))
        .unwrap()
        .expect("it was there");

    assert!(project.templates().contains(&template_id("renamed")));
    assert!(!project.templates().contains(&template_id("tpl")));
    assert_eq!(
        component_of(&project, &ViewKey::Project, "usage").definition,
        template_definition("renamed")
    );

    assert_eq!(
        kinds(&changes),
        ["rename-template", "set-component"],
        "the rename, then each component that pointed at it"
    );
}

#[test]
fn renaming_a_template_to_its_own_id_changes_nothing() {
    let mut changes = Vec::new();
    let result = project_with_template()
        .rename_template(&mut changes, &template_id("tpl"), template_id("tpl"))
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn renaming_a_template_onto_a_taken_id_is_refused() {
    let project = project_with_template()
        .set_template(&mut Vec::new(), template_id("other"))
        .unwrap();

    let error = project
        .rename_template(&mut Vec::new(), &template_id("tpl"), template_id("other"))
        .unwrap_err();

    assert!(matches!(
        error,
        CoreModelError::AlreadyExists {
            kind: IdKind::Template,
            ..
        }
    ));
}

// ---------------------------------------------------------------------------
// Exports
// ---------------------------------------------------------------------------

#[test]
fn exporting_a_member_records_what_it_points_at() {
    let mut changes = Vec::new();
    let project = project_with_template()
        .set_template_export(
            &mut changes,
            &template_id("tpl"),
            ExportKind::Member,
            export_id("out"),
            &component_id("{{id}}-inner"),
            "state",
        )
        .unwrap();

    assert_eq!(
        exports(&project, "tpl").members[&export_id("out")],
        MemberExport {
            component: component_id("{{id}}-inner"),
            member: String::from("state"),
        }
    );
    assert_eq!(kinds(&changes), ["set-template"]);
}

#[test]
fn exporting_a_member_the_component_does_not_have_is_refused() {
    let error = project_with_template()
        .set_template_export(
            &mut Vec::new(),
            &template_id("tpl"),
            ExportKind::Member,
            export_id("out"),
            &component_id("{{id}}-inner"),
            "nope",
        )
        .unwrap_err();

    assert!(matches!(error, CoreModelError::MemberNotFound { .. }));
}

#[test]
fn exporting_from_a_component_the_template_does_not_hold_is_refused() {
    let error = project_with_template()
        .set_template_export(
            &mut Vec::new(),
            &template_id("tpl"),
            ExportKind::Member,
            export_id("out"),
            &component_id("nope"),
            "state",
        )
        .unwrap_err();

    assert!(matches!(
        error,
        CoreModelError::NotFound {
            kind: IdKind::Component,
            ..
        }
    ));
}

#[test]
fn exporting_a_config_moves_it_from_the_component_to_its_users() {
    // This is the whole point of a config export: the inner component stops
    // carrying the value, and whoever instantiates the template supplies it.
    let (project, changes) = export_a_config(&project_with_template());

    assert!(
        config_value(&project, &template_view("tpl"), "{{id}}-inner", "label").is_none(),
        "the inner component no longer carries it"
    );
    assert_eq!(
        config_value(&project, &ViewKey::Project, "usage", "label"),
        Some(String::from("\"\"")),
        "and the component using the template gained it, at its default"
    );
    assert!(
        config_value(&project, &template_view("tpl"), "{{id}}-inner", "count").is_some(),
        "the config items that were not exported are untouched"
    );

    assert_eq!(
        kinds(&changes),
        ["set-component", "set-component", "set-template"],
        "the inner component, its user, then the export itself"
    );
}

#[test]
fn the_same_config_item_cannot_be_exported_twice() {
    // Two exports would both try to supply the same item.
    let (project, _) = export_a_config(&project_with_template());

    let error = project
        .set_template_export(
            &mut Vec::new(),
            &template_id("tpl"),
            ExportKind::Config,
            export_id("again"),
            &component_id("{{id}}-inner"),
            "label",
        )
        .unwrap_err();

    assert!(matches!(
        error,
        CoreModelError::ConfigAlreadyExported { .. }
    ));
}

#[test]
fn an_export_id_cannot_be_taken_twice() {
    let (project, _) = export_a_config(&project_with_template());

    let error = project
        .set_template_export(
            &mut Vec::new(),
            &template_id("tpl"),
            ExportKind::Config,
            export_id("label"),
            &component_id("{{id}}-inner"),
            "count",
        )
        .unwrap_err();

    assert!(matches!(
        error,
        CoreModelError::AlreadyExists {
            kind: IdKind::Export,
            ..
        }
    ));
}

#[test]
fn exporting_a_config_item_the_component_does_not_have_is_refused() {
    let error = project_with_template()
        .set_template_export(
            &mut Vec::new(),
            &template_id("tpl"),
            ExportKind::Config,
            export_id("out"),
            &component_id("{{id}}-inner"),
            "nope",
        )
        .unwrap_err();

    assert!(matches!(error, CoreModelError::ConfigNotFound { .. }));
}

#[test]
fn a_components_member_can_be_reached_through_the_template_that_exports_it() {
    // The recursion that makes a template answer like a plugin: the export
    // points at an inner component, whose own definition answers.
    let (project, _) = export_a_config(&project_with_template());

    let project = project
        .set_template_export(
            &mut Vec::new(),
            &template_id("tpl"),
            ExportKind::Member,
            export_id("out"),
            &component_id("{{id}}-inner"),
            "state",
        )
        .unwrap();

    assert_eq!(
        project
            .definition_member_value_type(&template_definition("tpl"), "out", MemberType::State)
            .unwrap(),
        "bool"
    );
    assert_eq!(
        project
            .definition_config_default(&template_definition("tpl"), "label")
            .unwrap(),
        serde_json::Value::String(String::new())
    );
}
