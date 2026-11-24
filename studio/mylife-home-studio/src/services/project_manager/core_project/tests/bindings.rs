//! Bindings: the one element whose id is derived from its contents rather
//! than authored.

use super::fixture::*;

#[test]
fn a_binding_is_stored_under_the_id_it_derives() {
    let mut changes = Vec::new();
    let project = project_with_components()
        .set_binding(
            &mut changes,
            &ViewKey::Project,
            binding("source", "state", "target", "action"),
        )
        .unwrap();

    assert_eq!(
        binding_ids(&project, &ViewKey::Project),
        ["source:state:target:action"]
    );
    assert_eq!(
        changed_bindings(&changes),
        ["set source:state:target:action"]
    );
}

#[test]
fn a_binding_onto_the_same_component_is_refused() {
    let error = project_with_components()
        .set_binding(
            &mut Vec::new(),
            &ViewKey::Project,
            binding("source", "state", "source", "action"),
        )
        .unwrap_err();

    assert_eq!(error, CoreModelError::BindingOnSelf);
}

#[test]
fn a_binding_between_mismatched_members_is_refused() {
    // `level` is a range, `action` takes a bool.
    let error = project_with_components()
        .set_binding(
            &mut Vec::new(),
            &ViewKey::Project,
            binding("source", "level", "target", "action"),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        CoreModelError::BindingMemberMismatch { .. }
    ));
}

#[test]
fn a_binding_has_to_join_a_state_to_an_action() {
    // Both ends exist, but in the wrong direction.
    let error = project_with_components()
        .set_binding(
            &mut Vec::new(),
            &ViewKey::Project,
            binding("source", "action", "target", "state"),
        )
        .unwrap_err();

    assert!(matches!(error, CoreModelError::MemberNotFound { .. }));
}

#[test]
fn a_binding_to_a_component_that_does_not_exist_is_refused() {
    let error = project_with_components()
        .set_binding(
            &mut Vec::new(),
            &ViewKey::Project,
            binding("source", "state", "nope", "action"),
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
fn the_same_binding_cannot_be_made_twice() {
    let error = project_with_binding()
        .set_binding(
            &mut Vec::new(),
            &ViewKey::Project,
            binding("source", "state", "target", "action"),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        CoreModelError::AlreadyExists {
            kind: IdKind::Binding,
            ..
        }
    ));
}

#[test]
fn a_refused_binding_leaves_the_project_alone() {
    let project = project_with_components();

    let mut changes = Vec::new();
    let result = project.set_binding(
        &mut changes,
        &ViewKey::Project,
        binding("source", "level", "target", "action"),
    );

    assert!(result.is_err());
    assert!(changes.is_empty(), "a refused operation emits nothing");
    assert!(binding_ids(&project, &ViewKey::Project).is_empty());
}

#[test]
fn clearing_a_binding_removes_it_and_says_so() {
    let project = project_with_binding();
    let id = binding("source", "state", "target", "action").id().unwrap();

    let mut changes = Vec::new();
    let project = project
        .clear_binding(&mut changes, &ViewKey::Project, &id)
        .unwrap()
        .expect("it was there");

    assert!(binding_ids(&project, &ViewKey::Project).is_empty());
    assert_eq!(kinds(&changes), ["clear-binding"]);
}

#[test]
fn clearing_an_unknown_binding_changes_nothing() {
    let mut changes = Vec::new();
    let result = project_with_binding()
        .clear_binding(
            &mut changes,
            &ViewKey::Project,
            &BindingId::new("a:b:c:d").unwrap(),
        )
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn bindings_live_in_the_view_they_were_made_in() {
    let project = project_with_binding()
        .set_template(&mut Vec::new(), template_id("tpl"))
        .unwrap();

    assert_eq!(binding_ids(&project, &ViewKey::Project).len(), 1);
    assert!(binding_ids(&project, &template_view("tpl")).is_empty());
}
