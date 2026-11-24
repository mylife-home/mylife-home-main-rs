//! Components: creating, moving, configuring, renaming and deleting them, in
//! the project view and inside a template.

use super::fixture::*;

#[test]
fn a_new_component_starts_from_its_plugin_config_template() {
    let mut changes = Vec::new();
    let project = project()
        .set_component(
            &mut changes,
            &ViewKey::Project,
            component_id("comp"),
            plugin_definition(),
            Position { x: 3, y: 4 },
        )
        .unwrap();

    let component = component_of(&project, &ViewKey::Project, "comp");

    assert_eq!(component.definition, plugin_definition());
    assert_eq!(component.position, Position { x: 3, y: 4 });
    assert!(!component.external);

    let config = component.config.as_ref().expect("a config");
    assert_eq!(config.get("label"), Some(&serde_json::json!("")));
    assert_eq!(config.get("count"), Some(&serde_json::json!(0)));

    assert_eq!(kinds(&changes), ["set-component"]);
}

#[test]
fn a_component_can_be_created_inside_a_template_too() {
    let project = project()
        .set_template(&mut Vec::new(), template_id("tpl"))
        .unwrap();

    let mut changes = Vec::new();
    let project = project
        .set_component(
            &mut changes,
            &template_view("tpl"),
            component_id("inner"),
            plugin_definition(),
            Position::default(),
        )
        .unwrap();

    assert_eq!(component_ids(&project, &template_view("tpl")), ["inner"]);
    assert!(
        component_ids(&project, &ViewKey::Project).is_empty(),
        "the project view is untouched"
    );
    assert_eq!(kinds(&changes), ["set-component"]);
}

#[test]
fn a_new_component_cannot_take_an_id_already_in_use() {
    let error = project_with_components()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("source"),
            plugin_definition(),
            Position::default(),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        CoreModelError::AlreadyExists {
            kind: IdKind::Component,
            ..
        }
    ));
}

#[test]
fn a_component_of_a_definition_that_does_not_exist_is_refused() {
    assert!(matches!(
        project().set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("comp"),
            ComponentDefinition::Plugin(PluginId::new("nope").unwrap()),
            Position::default(),
        ),
        Err(CoreModelError::NotFound {
            kind: IdKind::Plugin,
            ..
        })
    ));

    assert!(matches!(
        project().set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("comp"),
            template_definition("nope"),
            Position::default(),
        ),
        Err(CoreModelError::NotFound {
            kind: IdKind::Template,
            ..
        })
    ));
}

#[test]
fn a_template_cannot_contain_itself() {
    let error = project_with_template()
        .set_component(
            &mut Vec::new(),
            &template_view("tpl"),
            component_id("self"),
            template_definition("tpl"),
            Position::default(),
        )
        .unwrap_err();

    assert!(matches!(error, CoreModelError::CircularTemplate { .. }));
}

#[test]
fn a_template_cannot_contain_itself_through_another_template() {
    // `outer` already contains `tpl`, so putting `outer` inside `tpl` would
    // close the loop.
    let project = project_with_template()
        .set_template(&mut Vec::new(), template_id("outer"))
        .unwrap()
        .set_component(
            &mut Vec::new(),
            &template_view("outer"),
            component_id("uses-tpl"),
            template_definition("tpl"),
            Position::default(),
        )
        .unwrap();

    let error = project
        .set_component(
            &mut Vec::new(),
            &template_view("tpl"),
            component_id("uses-outer"),
            template_definition("outer"),
            Position::default(),
        )
        .unwrap_err();

    assert!(matches!(error, CoreModelError::CircularTemplate { .. }));
}

#[test]
fn moving_components_offsets_each_of_them() {
    let mut changes = Vec::new();
    let project = project_with_components()
        .move_components(
            &mut changes,
            &ViewKey::Project,
            &[component_id("source"), component_id("target")],
            Position { x: 5, y: -2 },
        )
        .unwrap();

    assert_eq!(
        component_of(&project, &ViewKey::Project, "source").position,
        Position { x: 5, y: -2 }
    );
    assert_eq!(
        component_of(&project, &ViewKey::Project, "target").position,
        Position { x: 15, y: 8 }
    );
    assert_eq!(
        changed_components(&changes),
        ["source", "target"],
        "one change per component moved"
    );
}

#[test]
fn moving_an_unknown_component_leaves_the_project_alone() {
    let project = project_with_components();

    let error = project
        .move_components(
            &mut Vec::new(),
            &ViewKey::Project,
            &[component_id("source"), component_id("nope")],
            Position { x: 5, y: 5 },
        )
        .unwrap_err();

    assert!(matches!(
        error,
        CoreModelError::NotFound {
            kind: IdKind::Component,
            ..
        }
    ));
    assert_eq!(
        component_of(&project, &ViewKey::Project, "source").position,
        Position { x: 0, y: 0 },
        "the caller still holds the project it started from"
    );
}

#[test]
fn configuring_a_component_sets_the_value() {
    let mut changes = Vec::new();
    let project = project_with_components()
        .configure_component(
            &mut changes,
            &ViewKey::Project,
            component_id("source"),
            "label",
            serde_json::json!("hello"),
        )
        .unwrap();

    assert_eq!(
        config_value(&project, &ViewKey::Project, "source", "label"),
        Some(String::from("\"hello\""))
    );
    assert_eq!(kinds(&changes), ["set-component"]);
}

#[test]
fn a_config_value_of_the_wrong_type_is_refused() {
    let error = project_with_components()
        .configure_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("source"),
            "count",
            serde_json::json!("not a number"),
        )
        .unwrap_err();

    assert!(matches!(error, CoreModelError::ConfigValueMismatch { .. }));
}

#[test]
fn a_config_item_that_does_not_exist_is_refused() {
    let error = project_with_components()
        .configure_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("source"),
            "nope",
            serde_json::json!("x"),
        )
        .unwrap_err();

    assert!(matches!(error, CoreModelError::ConfigNotFound { .. }));
}

#[test]
fn an_exported_config_item_cannot_be_set_on_the_component() {
    // The export owns it now; setting it here would resurrect a key the
    // template instance supplies.
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

    let error = project
        .configure_component(
            &mut Vec::new(),
            &template_view("tpl"),
            component_id("{{id}}-inner"),
            "label",
            serde_json::json!("hello"),
        )
        .unwrap_err();

    assert!(matches!(error, CoreModelError::ConfigNotSettable { .. }));
}

#[test]
fn renaming_a_component_rekeys_its_bindings() {
    // A binding has no id of its own, so renaming an end of it has to rebuild
    // the key. The old key goes, the new one arrives, and both are announced.
    let mut changes = Vec::new();
    let project = project_with_binding()
        .rename_component(
            &mut changes,
            &ViewKey::Project,
            &component_id("source"),
            component_id("renamed"),
        )
        .unwrap();

    assert_eq!(
        binding_ids(&project, &ViewKey::Project),
        ["renamed:state:target:action"]
    );
    assert_eq!(
        component_ids(&project, &ViewKey::Project),
        ["renamed", "target"]
    );

    assert_eq!(
        kinds(&changes),
        ["clear-binding", "rename-component", "set-binding"],
        "the old binding goes, the component is renamed, the new binding arrives"
    );
}

#[test]
fn a_binding_that_does_not_touch_the_renamed_component_keeps_its_key() {
    let project = project_with_binding()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("spare"),
            plugin_definition(),
            Position::default(),
        )
        .unwrap();

    let mut changes = Vec::new();
    let project = project
        .rename_component(
            &mut changes,
            &ViewKey::Project,
            &component_id("spare"),
            component_id("renamed"),
        )
        .unwrap();

    assert_eq!(
        binding_ids(&project, &ViewKey::Project),
        ["source:state:target:action"]
    );
    assert_eq!(kinds(&changes), ["rename-component"]);
}

#[test]
fn renaming_a_component_follows_it_into_the_template_exports() {
    let project = project_with_template()
        .set_template_export(
            &mut Vec::new(),
            &template_id("tpl"),
            ExportKind::Member,
            export_id("out"),
            &component_id("{{id}}-inner"),
            "state",
        )
        .unwrap();

    let mut changes = Vec::new();
    let project = project
        .rename_component(
            &mut changes,
            &template_view("tpl"),
            &component_id("{{id}}-inner"),
            component_id("{{id}}-renamed"),
        )
        .unwrap();

    let exports = project
        .templates()
        .get_required(&template_id("tpl"))
        .unwrap()
        .exports
        .clone();

    assert_eq!(
        exports.members[&export_id("out")].component,
        component_id("{{id}}-renamed")
    );
    assert_eq!(kinds(&changes), ["rename-component", "set-template"]);
}

#[test]
fn renaming_a_component_onto_a_taken_id_is_refused() {
    let error = project_with_components()
        .rename_component(
            &mut Vec::new(),
            &ViewKey::Project,
            &component_id("source"),
            component_id("target"),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        CoreModelError::AlreadyExists {
            kind: IdKind::Component,
            ..
        }
    ));
}

#[test]
fn deleting_a_component_takes_its_bindings() {
    let mut changes = Vec::new();
    let project = project_with_binding()
        .clear_components(&mut changes, &ViewKey::Project, &[component_id("source")])
        .unwrap();

    assert_eq!(component_ids(&project, &ViewKey::Project), ["target"]);
    assert!(binding_ids(&project, &ViewKey::Project).is_empty());

    assert_eq!(
        kinds(&changes),
        ["clear-binding", "clear-component"],
        "the binding goes first, so the client never sees a dangling one"
    );
}

#[test]
fn a_component_used_by_an_export_cannot_be_deleted() {
    let project = project_with_template()
        .set_template_export(
            &mut Vec::new(),
            &template_id("tpl"),
            ExportKind::Member,
            export_id("out"),
            &component_id("{{id}}-inner"),
            "state",
        )
        .unwrap();

    let mut changes = Vec::new();
    let error = project
        .clear_components(
            &mut changes,
            &template_view("tpl"),
            &[component_id("{{id}}-inner")],
        )
        .unwrap_err();

    assert!(matches!(error, CoreModelError::ComponentExported { .. }));
    assert!(changes.is_empty());
}

#[test]
fn a_selection_holding_one_undeletable_component_removes_none_of_them() {
    // Every component is checked before any is removed, so the project is
    // never left half applied.
    let project = project_with_template()
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
            &template_view("tpl"),
            component_id("{{id}}-spare"),
            plugin_definition(),
            Position::default(),
        )
        .unwrap();

    let mut changes = Vec::new();
    let error = project
        .clear_components(
            &mut changes,
            &template_view("tpl"),
            &[component_id("{{id}}-spare"), component_id("{{id}}-inner")],
        )
        .unwrap_err();

    assert!(matches!(error, CoreModelError::ComponentExported { .. }));
    assert_eq!(
        component_ids(&project, &template_view("tpl")),
        ["{{id}}-inner", "{{id}}-spare"],
        "the deletable one is still there"
    );
    assert!(
        changes.is_empty(),
        "and nothing was announced for it either: every component is checked \
         before any is removed"
    );
}

#[test]
fn deleting_an_unknown_component_is_refused() {
    assert!(matches!(
        project_with_components().clear_components(
            &mut Vec::new(),
            &ViewKey::Project,
            &[component_id("nope")]
        ),
        Err(CoreModelError::NotFound {
            kind: IdKind::Component,
            ..
        })
    ));
}
