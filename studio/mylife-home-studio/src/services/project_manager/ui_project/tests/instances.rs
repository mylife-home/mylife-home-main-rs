//! Template instances, and the binding operation: the one place where a
//! reference can point either at a component or at an export of the enclosing
//! template.

use std::collections::HashMap;
use std::sync::Arc;

use super::fixture::*;

fn instance_of(project: &Project, view: &ViewKey, id: &str) -> Arc<TemplateInstance> {
    project
        .view(view)
        .unwrap()
        .instances()
        .get_required(&instance_id(id))
        .unwrap()
        .clone()
}

fn bind(
    project: &Project,
    view: &ViewKey,
    instance: &str,
    export: &str,
    target: Option<MemberRef>,
) -> Result<Project, UiModelError> {
    project.set_template_instance_bindings(
        &mut Vec::new(),
        view,
        instance_id(instance),
        HashMap::from([(export_id(export), target)]),
    )
}

/// A template `outer` exporting a bool state `oe`, containing an instance of
/// `tpl`. The shape needed to bind one template's export to another's.
fn nested_project() -> Project {
    project_with_template()
        .new_template(&mut Vec::new(), template_id("outer"))
        .unwrap()
        .set_template_export(
            &mut Vec::new(),
            template_id("outer"),
            export_id("oe"),
            MemberType::State,
            "bool",
        )
        .unwrap()
        .new_template_instance(
            &mut Vec::new(),
            &template_view("outer"),
            instance_id("inst"),
            template_id("tpl"),
            0,
            0,
        )
        .unwrap()
}

// ---------------------------------------------------------------------------
// The instance itself
// ---------------------------------------------------------------------------

#[test]
fn a_new_instance_starts_with_one_unbound_slot_per_export() {
    let mut changes = Vec::new();
    let project = project_with_template()
        .new_window(&mut changes, window_id("win"))
        .unwrap()
        .new_template_instance(
            &mut changes,
            &window_view("win"),
            instance_id("inst"),
            template_id("tpl"),
            30,
            40,
        )
        .unwrap();

    let instance = instance_of(&project, &window_view("win"), "inst");
    assert_eq!(instance.template_id, template_id("tpl"));
    assert_eq!((instance.x, instance.y), (30, 40));
    assert_eq!(instance.bindings.len(), 1);
    assert_eq!(
        instance.bindings.get(&export_id("exp")),
        Some(&None),
        "the slot exists and is unbound"
    );
    assert_eq!(kinds(&changes), ["set-window", "set-window"]);
}

#[test]
fn instantiating_an_unknown_template_is_refused() {
    let error = project_with_window()
        .new_template_instance(
            &mut Vec::new(),
            &window_view("win"),
            instance_id("inst"),
            template_id("nope"),
            0,
            0,
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
fn a_new_instance_cannot_take_an_id_already_in_use() {
    let error = project_with_view_contents()
        .new_template_instance(
            &mut Vec::new(),
            &window_view("win"),
            instance_id("inst"),
            template_id("tpl"),
            0,
            0,
        )
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::AlreadyExists {
            kind: IdKind::TemplateInstance,
            ..
        }
    ));
}

#[test]
fn clearing_an_instance_removes_it_from_its_view() {
    let mut changes = Vec::new();
    let project = project_with_view_contents()
        .clear_template_instance(&mut changes, &window_view("win"), &instance_id("inst"))
        .unwrap()
        .expect("it was there");

    assert!(
        project
            .windows()
            .get_required(&window_id("win"))
            .unwrap()
            .templates
            .is_empty()
    );
    assert_eq!(kinds(&changes), ["set-window"]);
}

#[test]
fn clearing_an_unknown_instance_changes_nothing() {
    let mut changes = Vec::new();
    let result = project_with_window()
        .clear_template_instance(&mut changes, &window_view("win"), &instance_id("nope"))
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn renaming_an_instance_keeps_its_bindings() {
    let project = project_with_view_contents();
    let project = bind(
        &project,
        &window_view("win"),
        "inst",
        "exp",
        Some(component_ref("comp", "state")),
    )
    .unwrap();

    let project = project
        .rename_template_instance(
            &mut Vec::new(),
            &window_view("win"),
            &instance_id("inst"),
            &instance_id("renamed"),
        )
        .unwrap()
        .expect("it was there");

    let instance = instance_of(&project, &window_view("win"), "renamed");
    assert_eq!(
        instance.bindings.get(&export_id("exp")),
        Some(&Some(component_ref("comp", "state")))
    );
}

#[test]
fn renaming_an_instance_onto_a_taken_id_is_refused() {
    let project = project_with_view_contents()
        .new_template_instance(
            &mut Vec::new(),
            &window_view("win"),
            instance_id("other"),
            template_id("tpl"),
            0,
            0,
        )
        .unwrap();

    let error = project
        .rename_template_instance(
            &mut Vec::new(),
            &window_view("win"),
            &instance_id("inst"),
            &instance_id("other"),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::AlreadyExists {
            kind: IdKind::TemplateInstance,
            ..
        }
    ));
}

#[test]
fn a_copied_instance_is_offset_and_keeps_its_bindings() {
    let project = project_with_view_contents();
    let project = bind(
        &project,
        &window_view("win"),
        "inst",
        "exp",
        Some(component_ref("comp", "state")),
    )
    .unwrap();

    let project = project
        .clone_template_instance(
            &mut Vec::new(),
            &window_view("win"),
            &instance_id("inst"),
            instance_id("copy"),
        )
        .unwrap();

    let copy = instance_of(&project, &window_view("win"), "copy");
    assert_eq!((copy.x, copy.y), (40, 50), "offset from (30, 40)");
    assert_eq!(
        copy.bindings.get(&export_id("exp")),
        Some(&Some(component_ref("comp", "state")))
    );
}

#[test]
fn moving_an_instance_sets_its_position() {
    let mut changes = Vec::new();
    let project = project_with_view_contents()
        .move_template_instance(
            &mut changes,
            &window_view("win"),
            instance_id("inst"),
            100,
            200,
        )
        .unwrap();

    let instance = instance_of(&project, &window_view("win"), "inst");
    assert_eq!((instance.x, instance.y), (100, 200));
    assert_eq!(kinds(&changes), ["set-window"]);
}

#[test]
fn pointing_an_instance_at_another_template_resets_its_bindings() {
    // The bindings are keyed by the old template's exports, so none of them
    // means anything against the new one.
    let project = project_with_view_contents()
        .new_template(&mut Vec::new(), template_id("other"))
        .unwrap()
        .set_template_export(
            &mut Vec::new(),
            template_id("other"),
            export_id("oe"),
            MemberType::State,
            "bool",
        )
        .unwrap();

    let project = bind(
        &project,
        &window_view("win"),
        "inst",
        "exp",
        Some(component_ref("comp", "state")),
    )
    .unwrap();

    let project = project
        .set_template_instance_template(
            &mut Vec::new(),
            &window_view("win"),
            instance_id("inst"),
            template_id("other"),
        )
        .unwrap();

    let instance = instance_of(&project, &window_view("win"), "inst");
    assert_eq!(instance.template_id, template_id("other"));
    assert_eq!(
        instance.bindings.keys().collect::<Vec<_>>(),
        [&export_id("oe")],
        "slots for the new template's exports"
    );
    assert_eq!(instance.bindings.get(&export_id("oe")), Some(&None));
    assert_eq!(
        (instance.x, instance.y),
        (30, 40),
        "the instance stays where it was"
    );
}

#[test]
fn pointing_an_instance_at_an_unknown_template_is_refused() {
    let error = project_with_view_contents()
        .set_template_instance_template(
            &mut Vec::new(),
            &window_view("win"),
            instance_id("inst"),
            template_id("nope"),
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

// ---------------------------------------------------------------------------
// Bindings
// ---------------------------------------------------------------------------

#[test]
fn an_export_can_be_bound_to_a_component_member() {
    let project = project_with_view_contents();

    let mut changes = Vec::new();
    let project = project
        .set_template_instance_bindings(
            &mut changes,
            &window_view("win"),
            instance_id("inst"),
            HashMap::from([(export_id("exp"), Some(component_ref("comp", "state")))]),
        )
        .unwrap();

    assert_eq!(
        instance_of(&project, &window_view("win"), "inst")
            .bindings
            .get(&export_id("exp")),
        Some(&Some(component_ref("comp", "state")))
    );
    assert_eq!(kinds(&changes), ["set-window"]);
}

#[test]
fn a_binding_of_the_wrong_value_type_is_refused() {
    // `exp` carries a bool; `level` is a range.
    let error = bind(
        &project_with_view_contents(),
        &window_view("win"),
        "inst",
        "exp",
        Some(component_ref("comp", "level")),
    )
    .unwrap_err();

    assert!(matches!(error, UiModelError::BindingTypeMismatch { .. }));
}

#[test]
fn a_binding_of_the_wrong_member_type_is_refused() {
    // `exp` is a state; `action` is an action, even though both are bools.
    let error = bind(
        &project_with_view_contents(),
        &window_view("win"),
        "inst",
        "exp",
        Some(component_ref("comp", "action")),
    )
    .unwrap_err();

    assert!(matches!(error, UiModelError::MemberNotFound { .. }));
}

#[test]
fn a_binding_to_a_member_that_does_not_exist_is_refused() {
    let error = bind(
        &project_with_view_contents(),
        &window_view("win"),
        "inst",
        "exp",
        Some(component_ref("comp", "nope")),
    )
    .unwrap_err();

    assert!(matches!(error, UiModelError::MemberNotFound { .. }));
}

#[test]
fn a_binding_to_an_unknown_component_is_refused() {
    let error = bind(
        &project_with_view_contents(),
        &window_view("win"),
        "inst",
        "exp",
        Some(component_ref("nope", "state")),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::NotFound {
            kind: IdKind::Component,
            ..
        }
    ));
}

#[test]
fn an_unbound_binding_is_refused_rather_than_silently_applied() {
    // Unbinding is not what this operation is for, and `None` here is far more
    // likely to be a caller that lost the target than a deliberate request.
    let error = bind(
        &project_with_view_contents(),
        &window_view("win"),
        "inst",
        "exp",
        None,
    )
    .unwrap_err();

    assert!(matches!(error, UiModelError::BindingWithoutTarget { .. }));
}

#[test]
fn a_binding_for_an_export_the_template_does_not_have_is_refused() {
    let error = bind(
        &project_with_view_contents(),
        &window_view("win"),
        "inst",
        "nope",
        Some(component_ref("comp", "state")),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::NotFound {
            kind: IdKind::Export,
            ..
        }
    ));
}

#[test]
fn an_export_can_be_bound_to_an_export_of_the_enclosing_template() {
    let project = nested_project();

    let project = bind(
        &project,
        &template_view("outer"),
        "inst",
        "exp",
        Some(MemberRef::OwnerExport(export_id("oe"))),
    )
    .unwrap();

    assert_eq!(
        instance_of(&project, &template_view("outer"), "inst")
            .bindings
            .get(&export_id("exp")),
        Some(&Some(MemberRef::OwnerExport(export_id("oe"))))
    );
}

#[test]
fn binding_to_an_enclosing_export_needs_an_enclosing_template() {
    // In a window there is no enclosing template, so there is nothing for the
    // reference to mean.
    let error = bind(
        &project_with_view_contents(),
        &window_view("win"),
        "inst",
        "exp",
        Some(MemberRef::OwnerExport(export_id("exp"))),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::OwnerExportOutsideTemplate { .. }
    ));
}

#[test]
fn binding_to_an_enclosing_export_of_the_wrong_type_is_refused() {
    let project = nested_project()
        .set_template_export(
            &mut Vec::new(),
            template_id("outer"),
            export_id("ranged"),
            MemberType::State,
            "range[0;100]",
        )
        .unwrap();

    let error = bind(
        &project,
        &template_view("outer"),
        "inst",
        "exp",
        Some(MemberRef::OwnerExport(export_id("ranged"))),
    )
    .unwrap_err();

    assert!(matches!(error, UiModelError::BindingTypeMismatch { .. }));
}

#[test]
fn binding_to_an_enclosing_export_of_the_wrong_member_type_is_refused() {
    let project = nested_project()
        .set_template_export(
            &mut Vec::new(),
            template_id("outer"),
            export_id("act"),
            MemberType::Action,
            "bool",
        )
        .unwrap();

    let error = bind(
        &project,
        &template_view("outer"),
        "inst",
        "exp",
        Some(MemberRef::OwnerExport(export_id("act"))),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::ExportMemberTypeMismatch { .. }
    ));
}

#[test]
fn a_refused_binding_leaves_the_instance_alone() {
    let project = project_with_view_contents();

    let mut changes = Vec::new();
    let result = project.set_template_instance_bindings(
        &mut changes,
        &window_view("win"),
        instance_id("inst"),
        HashMap::from([(export_id("exp"), Some(component_ref("comp", "level")))]),
    );

    assert!(result.is_err());
    assert!(changes.is_empty(), "a refused operation emits nothing");
    assert_eq!(
        instance_of(&project, &window_view("win"), "inst")
            .bindings
            .get(&export_id("exp")),
        Some(&None),
        "and the caller still holds the project it started from"
    );
}
