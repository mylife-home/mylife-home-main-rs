//! Windows, their properties, and the two places a window id is referenced
//! from: a control action and the default-window map.

use std::collections::HashMap;

use super::fixture::*;

/// A window `target`, a window `win` holding a control whose primary action
/// opens `target`, and a default-window map pointing at `target` as well.
fn project_pointing_at(target: &str) -> Project {
    let project = project()
        .new_window(&mut Vec::new(), window_id(target))
        .unwrap()
        .new_window(&mut Vec::new(), window_id("win"))
        .unwrap()
        .new_control(
            &mut Vec::new(),
            &window_view("win"),
            control_id("ctl"),
            0,
            0,
            ControlKind::Display,
        )
        .unwrap();

    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        ControlProperties {
            primary_action: Patch::Set(Some(window_action(Some(window_id(target))))),
            ..Default::default()
        },
    );

    project.set_default_window(
        &mut Vec::new(),
        DefaultWindow::new(HashMap::from([(
            String::from("desktop"),
            Some(window_id(target)),
        )])),
    )
}

fn action_target(project: &Project, window: &str, control: &str) -> Option<WindowId> {
    project
        .windows()
        .get_required(&window_id(window))
        .unwrap()
        .controls
        .get_required(&control_id(control))
        .unwrap()
        .primary_action
        .as_ref()
        .expect("an action")
        .window
        .as_ref()
        .expect("a window action")
        .id
        .clone()
}

fn default_target(project: &Project, context: &str) -> Option<WindowId> {
    project
        .default_window()
        .iter()
        .find(|(key, _)| key.as_str() == context)
        .and_then(|(_, window)| window.clone())
}

#[test]
fn a_new_window_starts_empty_and_is_announced() {
    let mut changes = Vec::new();
    let project = project()
        .new_window(&mut changes, window_id("win"))
        .unwrap();

    let window = project.windows().get_required(&window_id("win")).unwrap();
    assert!(window.controls.is_empty());
    assert!(window.templates.is_empty());
    assert_eq!(kinds(&changes), ["set-window"]);
}

#[test]
fn a_new_window_cannot_take_an_id_already_in_use() {
    let project = project_with_window();

    let mut changes = Vec::new();
    let error = project
        .new_window(&mut changes, window_id("win"))
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::AlreadyExists {
            kind: IdKind::Window,
            ..
        }
    ));
    assert!(changes.is_empty());
}

#[test]
fn cloning_a_window_copies_what_is_in_it() {
    let project = project_with_view_contents();

    let mut changes = Vec::new();
    let project = project
        .clone_window(&mut changes, &window_id("win"), window_id("copy"))
        .unwrap();

    let copy = project.windows().get_required(&window_id("copy")).unwrap();
    assert!(copy.controls.contains(&control_id("ctl")));
    assert!(copy.templates.contains(&instance_id("inst")));

    assert!(
        project.windows().contains(&window_id("win")),
        "the original is left alone"
    );
    assert_eq!(kinds(&changes), ["set-window"]);
}

#[test]
fn cloning_an_unknown_window_is_refused() {
    let error = project()
        .clone_window(&mut Vec::new(), &window_id("nope"), window_id("copy"))
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
fn cloning_a_window_onto_a_taken_id_is_refused() {
    let project = project_with_window()
        .new_window(&mut Vec::new(), window_id("other"))
        .unwrap();

    let error = project
        .clone_window(&mut Vec::new(), &window_id("win"), window_id("other"))
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::AlreadyExists {
            kind: IdKind::Window,
            ..
        }
    ));
}

#[test]
fn setting_window_properties_writes_only_what_was_asked_for() {
    let project = project_with_window();

    let mut changes = Vec::new();
    let project = project
        .set_window_properties(
            &mut changes,
            window_id("win"),
            WindowProperties {
                height: Patch::Set(800),
                ..Default::default()
            },
        )
        .unwrap()
        .expect("the window changed");

    let window = project.windows().get_required(&window_id("win")).unwrap();
    assert_eq!(window.height, 800);
    assert_eq!(window.width, 500, "an unmentioned field is left alone");
    assert_eq!(kinds(&changes), ["set-window"]);
}

#[test]
fn a_properties_patch_that_sets_nothing_changes_nothing() {
    let project = project_with_window();

    let mut changes = Vec::new();
    let result = project
        .set_window_properties(&mut changes, window_id("win"), WindowProperties::default())
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn setting_properties_on_an_unknown_window_is_refused() {
    let error = project()
        .set_window_properties(
            &mut Vec::new(),
            window_id("nope"),
            WindowProperties {
                height: Patch::Set(800),
                ..Default::default()
            },
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
fn clearing_a_window_unsets_every_reference_to_it() {
    let project = project_pointing_at("target");

    let mut changes = Vec::new();
    let project = project
        .clear_window(&mut changes, &window_id("target"))
        .expect("it was there");

    assert!(!project.windows().contains(&window_id("target")));
    assert_eq!(action_target(&project, "win", "ctl"), None, "the action");
    assert_eq!(default_target(&project, "desktop"), None, "the default map");

    assert_eq!(
        kinds(&changes),
        ["clear-window", "set-window", "set-default-window"],
        "the removal, the view holding the action, and the default map"
    );
}

#[test]
fn clearing_an_unknown_window_changes_nothing() {
    let mut changes = Vec::new();
    let result = project().clear_window(&mut changes, &window_id("nope"));

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn renaming_a_window_follows_every_reference_to_it() {
    let project = project_pointing_at("target");

    let mut changes = Vec::new();
    let project = project
        .rename_window(&mut changes, &window_id("target"), window_id("renamed"))
        .unwrap()
        .expect("it was there");

    assert!(project.windows().contains(&window_id("renamed")));
    assert!(!project.windows().contains(&window_id("target")));
    assert_eq!(
        action_target(&project, "win", "ctl"),
        Some(window_id("renamed"))
    );
    assert_eq!(
        default_target(&project, "desktop"),
        Some(window_id("renamed"))
    );

    assert_eq!(
        kinds(&changes),
        ["rename-window", "set-window", "set-default-window"]
    );
}

#[test]
fn renaming_a_window_to_its_own_id_changes_nothing() {
    let project = project_with_window();

    let mut changes = Vec::new();
    let result = project
        .rename_window(&mut changes, &window_id("win"), window_id("win"))
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn renaming_a_window_onto_a_taken_id_is_refused() {
    let project = project_with_window()
        .new_window(&mut Vec::new(), window_id("other"))
        .unwrap();

    let mut changes = Vec::new();
    let error = project
        .rename_window(&mut changes, &window_id("win"), window_id("other"))
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::AlreadyExists {
            kind: IdKind::Window,
            ..
        }
    ));
    assert!(changes.is_empty());
}

#[test]
fn setting_the_default_window_map_replaces_it_wholesale() {
    let project = project_with_window();

    let mut changes = Vec::new();
    let project = project.set_default_window(
        &mut changes,
        DefaultWindow::new(HashMap::from([
            (String::from("desktop"), Some(window_id("win"))),
            (String::from("mobile"), None),
        ])),
    );

    assert_eq!(default_target(&project, "desktop"), Some(window_id("win")));
    assert_eq!(default_target(&project, "mobile"), None);
    assert_eq!(kinds(&changes), ["set-default-window"]);
}

#[test]
fn a_window_nothing_points_at_leaves_the_rest_alone() {
    let project = project_pointing_at("target")
        .new_window(&mut Vec::new(), window_id("lonely"))
        .unwrap();

    let mut changes = Vec::new();
    let project = project
        .clear_window(&mut changes, &window_id("lonely"))
        .expect("it was there");

    assert_eq!(
        action_target(&project, "win", "ctl"),
        Some(window_id("target")),
        "an unrelated action is untouched"
    );
    assert_eq!(kinds(&changes), ["clear-window"]);
}
