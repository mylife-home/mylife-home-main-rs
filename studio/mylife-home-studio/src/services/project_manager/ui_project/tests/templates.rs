//! Templates, their exports, and what a rename, a delete or a cleared export
//! does to the instances and controls that refer to them.

use std::collections::HashMap;

use super::fixture::*;

/// A template `inner` exporting `ie`, and a template `tpl` exporting `exp`
/// whose contents both refer to `exp`: a control display bound to it, and an
/// instance of `inner` whose own export is bound to it.
fn nested_templates() -> Project {
    let project = project()
        .new_template(&mut Vec::new(), template_id("inner"))
        .unwrap()
        .set_template_export(
            &mut Vec::new(),
            template_id("inner"),
            export_id("ie"),
            MemberType::State,
            "bool",
        )
        .unwrap()
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
        .new_control(
            &mut Vec::new(),
            &template_view("tpl"),
            control_id("ctl"),
            0,
            0,
            ControlKind::Display,
        )
        .unwrap();

    let project = set_control(
        &project,
        &template_view("tpl"),
        &control_id("ctl"),
        display_properties(Some(MemberRef::OwnerExport(export_id("exp")))),
    );

    project
        .new_template_instance(
            &mut Vec::new(),
            &template_view("tpl"),
            instance_id("inst"),
            template_id("inner"),
            0,
            0,
        )
        .unwrap()
        .set_template_instance_bindings(
            &mut Vec::new(),
            &template_view("tpl"),
            instance_id("inst"),
            HashMap::from([(
                export_id("ie"),
                Some(MemberRef::OwnerExport(export_id("exp"))),
            )]),
        )
        .unwrap()
}

fn display_member(project: &Project, template: &str, control: &str) -> Option<MemberRef> {
    project
        .templates()
        .get_required(&template_id(template))
        .unwrap()
        .controls
        .get_required(&control_id(control))
        .unwrap()
        .display
        .as_ref()
        .expect("a display control")
        .member
        .clone()
}

fn instance_binding(
    project: &Project,
    view: &ViewKey,
    instance: &str,
    export: &str,
) -> Option<MemberRef> {
    project
        .view(view)
        .unwrap()
        .instances()
        .get_required(&instance_id(instance))
        .unwrap()
        .bindings
        .get(&export_id(export))
        .expect("the slot exists")
        .clone()
}

// ---------------------------------------------------------------------------
// The template itself
// ---------------------------------------------------------------------------

#[test]
fn a_new_template_starts_empty_and_is_announced() {
    let mut changes = Vec::new();
    let project = project()
        .new_template(&mut changes, template_id("tpl"))
        .unwrap();

    let template = project
        .templates()
        .get_required(&template_id("tpl"))
        .unwrap();
    assert!(template.controls.is_empty());
    assert!(template.exports.is_empty());
    assert_eq!(kinds(&changes), ["set-template"]);
}

#[test]
fn a_new_template_cannot_take_an_id_already_in_use() {
    let error = project_with_template()
        .new_template(&mut Vec::new(), template_id("tpl"))
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::AlreadyExists {
            kind: IdKind::Template,
            ..
        }
    ));
}

#[test]
fn cloning_a_template_copies_its_contents_and_its_exports() {
    let project = nested_templates();

    let mut changes = Vec::new();
    let project = project
        .clone_template(&mut changes, &template_id("tpl"), template_id("copy"))
        .unwrap();

    let copy = project
        .templates()
        .get_required(&template_id("copy"))
        .unwrap();
    assert!(copy.exports.contains(&export_id("exp")));
    assert!(copy.controls.contains(&control_id("ctl")));
    assert!(copy.templates.contains(&instance_id("inst")));
    assert_eq!(kinds(&changes), ["set-template"]);
}

#[test]
fn setting_template_properties_writes_only_what_was_asked_for() {
    let mut changes = Vec::new();
    let project = project_with_template()
        .set_template_properties(
            &mut changes,
            template_id("tpl"),
            TemplateProperties {
                width: Patch::Set(300),
                ..Default::default()
            },
        )
        .unwrap()
        .expect("the template changed");

    let template = project
        .templates()
        .get_required(&template_id("tpl"))
        .unwrap();
    assert_eq!(template.width, 300);
    assert_eq!(template.height, 500, "an unmentioned field is left alone");
    assert_eq!(kinds(&changes), ["set-template"]);
}

#[test]
fn a_template_properties_patch_that_sets_nothing_changes_nothing() {
    let mut changes = Vec::new();
    let result = project_with_template()
        .set_template_properties(
            &mut changes,
            template_id("tpl"),
            TemplateProperties::default(),
        )
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn deleting_a_template_takes_its_instances_with_it() {
    // An instance of a template that no longer exists has nothing to deploy, so
    // it goes rather than being left dangling.
    let project = project_with_view_contents();

    let mut changes = Vec::new();
    let project = project
        .clear_template(&mut changes, &template_id("tpl"))
        .expect("it was there");

    assert!(!project.templates().contains(&template_id("tpl")));
    assert!(
        project
            .windows()
            .get_required(&window_id("win"))
            .unwrap()
            .templates
            .is_empty(),
        "the instance went with it"
    );
    assert_eq!(kinds(&changes), ["clear-template", "set-window"]);
}

#[test]
fn clearing_an_unknown_template_changes_nothing() {
    let mut changes = Vec::new();
    let result = project().clear_template(&mut changes, &template_id("nope"));

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn renaming_a_template_follows_its_instances() {
    let project = project_with_view_contents();

    let mut changes = Vec::new();
    let project = project
        .rename_template(&mut changes, &template_id("tpl"), template_id("renamed"))
        .unwrap()
        .expect("it was there");

    let instance = project
        .windows()
        .get_required(&window_id("win"))
        .unwrap()
        .templates
        .get_required(&instance_id("inst"))
        .unwrap()
        .clone();

    assert_eq!(instance.template_id, template_id("renamed"));
    assert_eq!(kinds(&changes), ["rename-template", "set-window"]);
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
        .new_template(&mut Vec::new(), template_id("other"))
        .unwrap();

    let error = project
        .rename_template(&mut Vec::new(), &template_id("tpl"), template_id("other"))
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::AlreadyExists {
            kind: IdKind::Template,
            ..
        }
    ));
}

// ---------------------------------------------------------------------------
// Exports
// ---------------------------------------------------------------------------

#[test]
fn an_export_records_what_it_carries() {
    let mut changes = Vec::new();
    let project = project()
        .new_template(&mut changes, template_id("tpl"))
        .unwrap()
        .set_template_export(
            &mut changes,
            template_id("tpl"),
            export_id("exp"),
            MemberType::Action,
            "range[0;10]",
        )
        .unwrap();

    let export = project
        .templates()
        .get_required(&template_id("tpl"))
        .unwrap()
        .exports
        .get_required(&export_id("exp"))
        .unwrap()
        .clone();

    assert_eq!(export.member_type, MemberType::Action);
    assert_eq!(export.value_type, "range[0;10]");
    assert_eq!(kinds(&changes), ["set-template", "set-template"]);
}

#[test]
fn exporting_from_an_unknown_template_is_refused() {
    let error = project()
        .set_template_export(
            &mut Vec::new(),
            template_id("nope"),
            export_id("exp"),
            MemberType::State,
            "bool",
        )
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::NotFound {
            kind: IdKind::Template,
            ..
        }
    ));
}

#[test]
fn clearing_an_export_clears_its_users_in_one_change() {
    let project = nested_templates();

    let mut changes = Vec::new();
    let project = project
        .clear_template_export(&mut changes, template_id("tpl"), &export_id("exp"))
        .unwrap()
        .expect("it was there");

    let template = project
        .templates()
        .get_required(&template_id("tpl"))
        .unwrap();
    assert!(!template.exports.contains(&export_id("exp")));

    assert_eq!(
        display_member(&project, "tpl", "ctl"),
        None,
        "the control inside the template"
    );
    assert_eq!(
        instance_binding(&project, &template_view("tpl"), "inst", "ie"),
        None,
        "the instance nested inside the template"
    );

    assert_eq!(
        kinds(&changes),
        ["set-template"],
        "one change for the template, however many references it held"
    );
}

#[test]
fn clearing_an_unknown_export_changes_nothing() {
    let mut changes = Vec::new();
    let result = project_with_template()
        .clear_template_export(&mut changes, template_id("tpl"), &export_id("nope"))
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn clearing_an_export_leaves_instances_in_other_views_holding_a_stale_slot() {
    // An instance living outside the template that owns the export keeps a
    // binding keyed by an export that is gone. Deploy is what reports it; this
    // test pins that it is deploy's job and not this operation's.
    let project = project_with_view_contents();

    let project = project
        .clear_template_export(&mut Vec::new(), template_id("tpl"), &export_id("exp"))
        .unwrap()
        .expect("it was there");

    let instance = project
        .windows()
        .get_required(&window_id("win"))
        .unwrap()
        .templates
        .get_required(&instance_id("inst"))
        .unwrap()
        .clone();

    assert!(
        instance.bindings.contains_key(&export_id("exp")),
        "the slot survives the export it was keyed by"
    );
}

#[test]
fn bulk_patterns_are_recorded_on_the_exports_they_name() {
    let mut changes = Vec::new();
    let project = project_with_template()
        .set_template_bulk_patterns(
            &mut changes,
            template_id("tpl"),
            &HashMap::from([(export_id("exp"), String::from("{{id}}-suffix"))]),
        )
        .unwrap()
        .expect("the template changed");

    assert_eq!(
        project
            .templates()
            .get_required(&template_id("tpl"))
            .unwrap()
            .exports
            .get_required(&export_id("exp"))
            .unwrap()
            .bulk_pattern,
        Some(String::from("{{id}}-suffix"))
    );
    assert_eq!(kinds(&changes), ["set-template"]);
}

#[test]
fn a_bulk_pattern_for_an_unknown_export_is_refused() {
    let mut changes = Vec::new();
    let error = project_with_template()
        .set_template_bulk_patterns(
            &mut changes,
            template_id("tpl"),
            &HashMap::from([(export_id("nope"), String::from("x"))]),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::NotFound {
            kind: IdKind::Export,
            ..
        }
    ));
    assert!(changes.is_empty());
}

#[test]
fn an_empty_set_of_bulk_patterns_changes_nothing() {
    let mut changes = Vec::new();
    let result = project_with_template()
        .set_template_bulk_patterns(&mut changes, template_id("tpl"), &HashMap::new())
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}
