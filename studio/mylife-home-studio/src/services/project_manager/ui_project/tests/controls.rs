//! Controls: creating, copying, renaming and patching them, in a window and in
//! a template.

use super::fixture::*;

fn control_of(project: &Project, view: &ViewKey, id: &str) -> std::sync::Arc<Control> {
    project
        .view(view)
        .unwrap()
        .controls()
        .get_required(&control_id(id))
        .unwrap()
        .clone()
}

#[test]
fn a_new_display_control_starts_with_a_display_and_nothing_else() {
    let mut changes = Vec::new();
    let project = project_with_window()
        .new_control(
            &mut changes,
            &window_view("win"),
            control_id("ctl"),
            10,
            20,
            ControlKind::Display,
        )
        .unwrap();

    let control = control_of(&project, &window_view("win"), "ctl");
    assert!(control.display.is_some());
    assert!(control.text.is_none());
    assert_eq!((control.x, control.y), (10, 20));
    assert!(control.primary_action.is_none());
    assert_eq!(kinds(&changes), ["set-window"]);
}

#[test]
fn a_new_text_control_starts_with_a_text_and_nothing_else() {
    let project = project_with_window()
        .new_control(
            &mut Vec::new(),
            &window_view("win"),
            control_id("ctl"),
            0,
            0,
            ControlKind::Text,
        )
        .unwrap();

    let control = control_of(&project, &window_view("win"), "ctl");
    assert!(control.text.is_some());
    assert!(control.display.is_none());
}

#[test]
fn a_control_can_be_created_in_a_template_too() {
    // The operation takes a view key, so a window and a template are the same
    // call with a different key.
    let mut changes = Vec::new();
    let project = project_with_template()
        .new_control(
            &mut changes,
            &template_view("tpl"),
            control_id("ctl"),
            0,
            0,
            ControlKind::Display,
        )
        .unwrap();

    assert!(
        project
            .templates()
            .get_required(&template_id("tpl"))
            .unwrap()
            .controls
            .contains(&control_id("ctl"))
    );
    assert_eq!(kinds(&changes), ["set-template"]);
}

#[test]
fn a_new_control_cannot_take_an_id_already_in_use() {
    let error = project_with_view_contents()
        .new_control(
            &mut Vec::new(),
            &window_view("win"),
            control_id("ctl"),
            0,
            0,
            ControlKind::Display,
        )
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::AlreadyExists {
            kind: IdKind::Control,
            ..
        }
    ));
}

#[test]
fn creating_a_control_in_an_unknown_view_is_refused() {
    let error = project()
        .new_control(
            &mut Vec::new(),
            &window_view("nope"),
            control_id("ctl"),
            0,
            0,
            ControlKind::Display,
        )
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::NotFound {
            kind: IdKind::Window,
            ..
        }
    ));
}

#[test]
fn clearing_a_control_removes_it_from_its_view() {
    let mut changes = Vec::new();
    let project = project_with_view_contents()
        .clear_control(&mut changes, &window_view("win"), &control_id("ctl"))
        .unwrap()
        .expect("it was there");

    assert!(
        !project
            .windows()
            .get_required(&window_id("win"))
            .unwrap()
            .controls
            .contains(&control_id("ctl"))
    );
    assert_eq!(kinds(&changes), ["set-window"]);
}

#[test]
fn clearing_an_unknown_control_changes_nothing() {
    let mut changes = Vec::new();
    let result = project_with_window()
        .clear_control(&mut changes, &window_view("win"), &control_id("nope"))
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn renaming_a_control_keeps_what_is_in_it() {
    let project = project_with_view_contents();
    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        display_properties(Some(component_ref("comp", "state"))),
    );

    let mut changes = Vec::new();
    let project = project
        .rename_control(
            &mut changes,
            &window_view("win"),
            &control_id("ctl"),
            &control_id("renamed"),
        )
        .unwrap()
        .expect("it was there");

    let control = control_of(&project, &window_view("win"), "renamed");
    assert_eq!(
        control.display.as_ref().unwrap().member,
        Some(component_ref("comp", "state"))
    );
    assert_eq!(kinds(&changes), ["set-window"]);
}

#[test]
fn renaming_a_control_to_its_own_id_changes_nothing() {
    let mut changes = Vec::new();
    let result = project_with_view_contents()
        .rename_control(
            &mut changes,
            &window_view("win"),
            &control_id("ctl"),
            &control_id("ctl"),
        )
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn renaming_a_control_onto_a_taken_id_is_refused() {
    let project = project_with_view_contents()
        .new_control(
            &mut Vec::new(),
            &window_view("win"),
            control_id("other"),
            0,
            0,
            ControlKind::Display,
        )
        .unwrap();

    let error = project
        .rename_control(
            &mut Vec::new(),
            &window_view("win"),
            &control_id("ctl"),
            &control_id("other"),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::AlreadyExists {
            kind: IdKind::Control,
            ..
        }
    ));
}

#[test]
fn a_copy_in_the_same_view_is_offset_from_the_original() {
    let project = project_with_view_contents();

    let project = project
        .clone_control(
            &mut Vec::new(),
            &window_view("win"),
            &control_id("ctl"),
            &window_view("win"),
            control_id("copy"),
        )
        .unwrap();

    let original = control_of(&project, &window_view("win"), "ctl");
    let copy = control_of(&project, &window_view("win"), "copy");

    assert_eq!((original.x, original.y), (10, 20));
    assert_eq!(
        (copy.x, copy.y),
        (20, 30),
        "offset, so the copy does not land exactly on top of the original"
    );
}

#[test]
fn a_copy_into_another_view_is_placed_and_only_that_view_is_written() {
    let project = project_with_view_contents();
    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        display_properties(Some(component_ref("comp", "state"))),
    );

    let source_before = project
        .windows()
        .get_required(&window_id("win"))
        .unwrap()
        .clone();

    let mut changes = Vec::new();
    let project = project
        .clone_control(
            &mut changes,
            &window_view("win"),
            &control_id("ctl"),
            &template_view("tpl"),
            control_id("copy"),
        )
        .unwrap();

    let copy = control_of(&project, &template_view("tpl"), "copy");
    assert_eq!(
        (copy.x, copy.y),
        (10, 10),
        "placed, since there is nothing to be offset from in the target view"
    );
    assert_eq!(
        copy.display.as_ref().unwrap().member,
        Some(component_ref("comp", "state")),
        "the copy keeps the original's references"
    );

    assert!(
        std::sync::Arc::ptr_eq(
            &source_before,
            project.windows().get_required(&window_id("win")).unwrap()
        ),
        "the source view is untouched"
    );
    assert_eq!(
        kinds(&changes),
        ["set-template"],
        "and only the target view is announced"
    );
}

#[test]
fn copying_an_unknown_control_is_refused() {
    let error = project_with_view_contents()
        .clone_control(
            &mut Vec::new(),
            &window_view("win"),
            &control_id("nope"),
            &window_view("win"),
            control_id("copy"),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::NotFound {
            kind: IdKind::Control,
            ..
        }
    ));
}

#[test]
fn patching_a_control_writes_only_the_fields_the_patch_names() {
    let project = project_with_view_contents();

    let mut changes = Vec::new();
    let project = project
        .set_control_properties(
            &mut changes,
            &window_view("win"),
            control_id("ctl"),
            ControlProperties {
                x: Patch::Set(99),
                primary_action: Patch::Set(Some(component_action(Some(component_ref(
                    "comp", "action",
                ))))),
                ..Default::default()
            },
        )
        .unwrap()
        .expect("the control changed");

    let control = control_of(&project, &window_view("win"), "ctl");
    assert_eq!(control.x, 99);
    assert_eq!(control.y, 20, "an unmentioned field is left alone");
    assert!(control.display.is_some(), "and so is the display");
    assert_eq!(
        control
            .primary_action
            .as_ref()
            .unwrap()
            .component
            .as_ref()
            .unwrap()
            .member,
        Some(component_ref("comp", "action"))
    );
    assert_eq!(kinds(&changes), ["set-window"]);
}

#[test]
fn a_control_patch_can_clear_a_field_as_well_as_set_one() {
    // `Patch<Option<T>>` is two decisions: not mentioned, and explicitly unset.
    let project = project_with_view_contents();

    let project = project
        .set_control_properties(
            &mut Vec::new(),
            &window_view("win"),
            control_id("ctl"),
            ControlProperties {
                display: Patch::Set(None),
                text: Patch::Set(Some(text_with(Vec::new()))),
                ..Default::default()
            },
        )
        .unwrap()
        .expect("the control changed");

    let control = control_of(&project, &window_view("win"), "ctl");
    assert!(control.display.is_none(), "explicitly cleared");
    assert!(control.text.is_some());
}

#[test]
fn a_control_patch_that_sets_nothing_changes_nothing() {
    let mut changes = Vec::new();
    let result = project_with_view_contents()
        .set_control_properties(
            &mut changes,
            &window_view("win"),
            control_id("ctl"),
            ControlProperties::default(),
        )
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn patching_an_unknown_control_is_refused() {
    let error = project_with_window()
        .set_control_properties(
            &mut Vec::new(),
            &window_view("win"),
            control_id("nope"),
            ControlProperties {
                x: Patch::Set(1),
                ..Default::default()
            },
        )
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::NotFound {
            kind: IdKind::Control,
            ..
        }
    ));
}
