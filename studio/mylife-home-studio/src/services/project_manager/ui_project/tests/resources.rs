//! Resources, and the references to them that a rename or a delete has to
//! follow: a window background, a display's default image, and the images in a
//! display's map.

use std::sync::Arc;

use super::fixture::*;

fn image() -> Resource {
    Resource {
        mime: String::from("image/png"),
        data: String::new(),
    }
}

fn map_item(resource: &str) -> ControlDisplayMapItem {
    ControlDisplayMapItem {
        min: None,
        max: None,
        value: serde_json::Value::Bool(true),
        resource: Some(resource_id(resource)),
    }
}

/// A window using `old` as its background, holding a control whose display uses
/// it both as the default image and in its map.
fn project_using(resource: &str) -> Project {
    let project = project()
        .set_resource(&mut Vec::new(), resource_id(resource), image())
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
            display: Patch::Set(Some(Arc::new(ControlDisplay {
                member: None,
                default_resource: Some(resource_id(resource)),
                map: vec![map_item(resource)],
            }))),
            ..Default::default()
        },
    );

    project
        .set_window_properties(
            &mut Vec::new(),
            window_id("win"),
            WindowProperties {
                background_resource: Patch::Set(Some(resource_id(resource))),
                ..Default::default()
            },
        )
        .unwrap()
        .expect("the window changed")
}

fn display_of(project: &Project, window: &str, control: &str) -> Arc<ControlDisplay> {
    project
        .windows()
        .get_required(&window_id(window))
        .unwrap()
        .controls
        .get_required(&control_id(control))
        .unwrap()
        .display
        .clone()
        .expect("a display control")
}

#[test]
fn setting_a_resource_stores_it_and_says_so() {
    let mut changes = Vec::new();
    let project = project().set_resource(&mut changes, resource_id("img"), image());

    assert_eq!(project.resources().len(), 1);
    assert_eq!(
        project
            .resources()
            .get_required(&resource_id("img"))
            .unwrap()
            .mime,
        "image/png"
    );
    assert_eq!(kinds(&changes), ["set-resource"]);
}

#[test]
fn setting_a_resource_twice_replaces_it() {
    let project = project().set_resource(&mut Vec::new(), resource_id("img"), image());

    let mut changes = Vec::new();
    let project = project.set_resource(
        &mut changes,
        resource_id("img"),
        Resource {
            mime: String::from("image/jpeg"),
            data: String::new(),
        },
    );

    assert_eq!(project.resources().len(), 1, "replaced, not added");
    assert_eq!(
        project
            .resources()
            .get_required(&resource_id("img"))
            .unwrap()
            .mime,
        "image/jpeg"
    );
    assert_eq!(kinds(&changes), ["set-resource"]);
}

#[test]
fn clearing_a_resource_removes_it_and_says_so() {
    let project = project().set_resource(&mut Vec::new(), resource_id("img"), image());

    let mut changes = Vec::new();
    let project = project
        .clear_resource(&mut changes, &resource_id("img"))
        .expect("it was there");

    assert!(project.resources().is_empty());
    assert_eq!(kinds(&changes), ["clear-resource"]);
}

#[test]
fn clearing_an_unknown_resource_changes_nothing() {
    let mut changes = Vec::new();
    let result = project().clear_resource(&mut changes, &resource_id("nope"));

    assert!(result.is_none());
    assert!(changes.is_empty(), "nothing happened, nothing is emitted");
}

#[test]
fn clearing_a_resource_unsets_every_reference_to_it() {
    let project = project_using("img");

    let mut changes = Vec::new();
    let project = project
        .clear_resource(&mut changes, &resource_id("img"))
        .expect("it was there");

    let window = project.windows().get_required(&window_id("win")).unwrap();
    assert_eq!(window.background_resource, None, "window background");

    let display = display_of(&project, "win", "ctl");
    assert_eq!(display.default_resource, None, "default image");
    assert_eq!(display.map[0].resource, None, "map entry");

    assert_eq!(
        kinds(&changes),
        ["clear-resource", "set-window"],
        "the removal, then the view it reached into"
    );
}

#[test]
fn renaming_a_resource_follows_every_reference_to_it() {
    let project = project_using("old");

    let mut changes = Vec::new();
    let project = project
        .rename_resource(&mut changes, &resource_id("old"), resource_id("new"))
        .unwrap()
        .expect("it was there");

    assert!(project.resources().contains(&resource_id("new")));
    assert!(!project.resources().contains(&resource_id("old")));

    let window = project.windows().get_required(&window_id("win")).unwrap();
    assert_eq!(window.background_resource, Some(resource_id("new")));

    let display = display_of(&project, "win", "ctl");
    assert_eq!(display.default_resource, Some(resource_id("new")));
    assert_eq!(display.map[0].resource, Some(resource_id("new")));

    assert_eq!(kinds(&changes), ["rename-resource", "set-window"]);
}

#[test]
fn renaming_a_resource_to_its_own_id_changes_nothing() {
    let project = project_using("img");

    let mut changes = Vec::new();
    let result = project
        .rename_resource(&mut changes, &resource_id("img"), resource_id("img"))
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn renaming_a_resource_onto_a_taken_id_is_refused() {
    let project = project()
        .set_resource(&mut Vec::new(), resource_id("a"), image())
        .set_resource(&mut Vec::new(), resource_id("b"), image());

    let mut changes = Vec::new();
    let error = project
        .rename_resource(&mut changes, &resource_id("a"), resource_id("b"))
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::AlreadyExists {
            kind: IdKind::Resource,
            ..
        }
    ));
    assert!(changes.is_empty(), "a refused operation emits nothing");
}

#[test]
fn renaming_an_unknown_resource_is_refused() {
    let error = project()
        .rename_resource(&mut Vec::new(), &resource_id("nope"), resource_id("new"))
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::NotFound {
            kind: IdKind::Resource,
            ..
        }
    ));
}

#[test]
fn a_resource_a_view_does_not_use_leaves_that_view_alone() {
    let project = project_using("img").set_resource(&mut Vec::new(), resource_id("other"), image());

    let before = project
        .windows()
        .get_required(&window_id("win"))
        .unwrap()
        .clone();

    let mut changes = Vec::new();
    let project = project
        .clear_resource(&mut changes, &resource_id("other"))
        .expect("it was there");

    let after = project.windows().get_required(&window_id("win")).unwrap();

    assert!(
        Arc::ptr_eq(&before, after),
        "an untouched view keeps its pointer, so nothing downstream rebuilds it"
    );
    assert_eq!(kinds(&changes), ["clear-resource"], "and emits nothing");
}
