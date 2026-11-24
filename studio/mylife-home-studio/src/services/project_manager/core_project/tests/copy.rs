//! Copying components into another view, and the placeholder that keeps the
//! copies apart when the target template is used more than once.

use super::fixture::*;

/// `project_with_binding()` plus an empty template to copy into.
fn project_with_target() -> Project {
    project_with_binding()
        .set_template(&mut Vec::new(), template_id("tpl"))
        .unwrap()
}

#[test]
fn copies_are_named_for_the_component_that_will_instantiate_the_template() {
    // `{{id}}` is substituted at deploy time with the id of whatever
    // instantiates the template, so two uses of it do not collide.
    let mut changes = Vec::new();
    let (project, components, bindings) = project_with_target()
        .copy_components(
            &mut changes,
            &ViewKey::Project,
            &[component_id("source"), component_id("target")],
            &template_view("tpl"),
        )
        .unwrap();

    assert_eq!((components, bindings), (2, 1));
    assert_eq!(
        component_ids(&project, &template_view("tpl")),
        ["{{id}}-source", "{{id}}-target"]
    );
    assert_eq!(
        binding_ids(&project, &template_view("tpl")),
        ["{{id}}-source:state:{{id}}-target:action"],
        "the binding was re-derived from the copies' ids"
    );
    assert_eq!(
        component_ids(&project, &ViewKey::Project),
        ["source", "target"],
        "the source view is left alone"
    );
}

#[test]
fn only_the_bindings_wholly_inside_the_selection_come_along() {
    // A binding with one end outside the selection has nothing to attach to.
    let (project, components, bindings) = project_with_target()
        .copy_components(
            &mut Vec::new(),
            &ViewKey::Project,
            &[component_id("source")],
            &template_view("tpl"),
        )
        .unwrap();

    assert_eq!((components, bindings), (1, 0));
    assert!(binding_ids(&project, &template_view("tpl")).is_empty());
}

#[test]
fn copying_a_view_onto_itself_is_refused() {
    let error = project_with_target()
        .copy_components(
            &mut Vec::new(),
            &ViewKey::Project,
            &[component_id("source")],
            &ViewKey::Project,
        )
        .unwrap_err();

    assert_eq!(error, CoreModelError::CopyToSelf);
}

#[test]
fn a_component_instantiating_a_template_cannot_be_copied() {
    let project = project_with_template()
        .set_template(&mut Vec::new(), template_id("other"))
        .unwrap();

    let error = project
        .copy_components(
            &mut Vec::new(),
            &ViewKey::Project,
            &[component_id("usage")],
            &template_view("other"),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        CoreModelError::NotCopyable {
            reason: "it instantiates a template",
            ..
        }
    ));
}

#[test]
fn copying_the_same_selection_twice_is_refused() {
    let (project, ..) = project_with_target()
        .copy_components(
            &mut Vec::new(),
            &ViewKey::Project,
            &[component_id("source")],
            &template_view("tpl"),
        )
        .unwrap();

    let error = project
        .copy_components(
            &mut Vec::new(),
            &ViewKey::Project,
            &[component_id("source")],
            &template_view("tpl"),
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
fn copying_an_unknown_component_is_refused() {
    assert!(matches!(
        project_with_target().copy_components(
            &mut Vec::new(),
            &ViewKey::Project,
            &[component_id("nope")],
            &template_view("tpl"),
        ),
        Err(CoreModelError::NotFound {
            kind: IdKind::Component,
            ..
        })
    ));
}

#[test]
fn a_copy_announces_each_component_and_each_binding() {
    let mut changes = Vec::new();
    project_with_target()
        .copy_components(
            &mut changes,
            &ViewKey::Project,
            &[component_id("source"), component_id("target")],
            &template_view("tpl"),
        )
        .unwrap();

    assert_eq!(
        kinds(&changes),
        ["set-component", "set-component", "set-binding"]
    );
}
