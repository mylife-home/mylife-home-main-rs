//! Styles, the lists that name them, and what a rename or a delete has to do
//! to those lists.

use super::fixture::*;

fn css() -> StyleDefinition {
    StyleDefinition {
        properties: serde_json::json!({ "color": "red" }),
    }
}

/// A window styled `old`, holding a control styled `old` too.
fn project_using(style: &str) -> Project {
    let project = project()
        .set_style(&mut Vec::new(), style_id(style), css())
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
            style: Patch::Set(StyleList::new(vec![style_id(style)])),
            ..Default::default()
        },
    );

    project
        .set_window_properties(
            &mut Vec::new(),
            window_id("win"),
            WindowProperties {
                style: Patch::Set(StyleList::new(vec![style_id(style)])),
                ..Default::default()
            },
        )
        .unwrap()
        .expect("the window changed")
}

fn control_style(project: &Project, window: &str, control: &str) -> StyleList {
    project
        .windows()
        .get_required(&window_id(window))
        .unwrap()
        .controls
        .get_required(&control_id(control))
        .unwrap()
        .style
        .clone()
}

#[test]
fn a_style_list_sorts_itself() {
    // Sorting in the constructor is what makes two lists with the same members
    // compare equal, so nothing downstream has to normalize before comparing.
    let list = StyleList::new(vec![style_id("c"), style_id("a"), style_id("b")]);

    assert_eq!(
        list.as_slice(),
        [style_id("a"), style_id("b"), style_id("c")]
    );
    assert_eq!(
        list,
        StyleList::new(vec![style_id("b"), style_id("a"), style_id("c")])
    );
}

#[test]
fn setting_a_style_stores_it_and_says_so() {
    let mut changes = Vec::new();
    let project = project().set_style(&mut changes, style_id("red"), css());

    assert_eq!(project.styles().len(), 1);
    assert_eq!(kinds(&changes), ["set-style"]);
}

#[test]
fn setting_a_style_twice_replaces_it() {
    let project = project().set_style(&mut Vec::new(), style_id("red"), css());

    let mut changes = Vec::new();
    let project = project.set_style(
        &mut changes,
        style_id("red"),
        StyleDefinition {
            properties: serde_json::json!({ "color": "blue" }),
        },
    );

    assert_eq!(project.styles().len(), 1);
    assert_eq!(
        project
            .styles()
            .get_required(&style_id("red"))
            .unwrap()
            .properties,
        serde_json::json!({ "color": "blue" })
    );
    assert_eq!(kinds(&changes), ["set-style"]);
}

#[test]
fn clearing_a_style_drops_it_from_every_list_that_names_it() {
    let project = project_using("red");

    let mut changes = Vec::new();
    let project = project
        .clear_style(&mut changes, &style_id("red"))
        .expect("it was there");

    assert!(project.styles().is_empty());
    assert!(
        project
            .windows()
            .get_required(&window_id("win"))
            .unwrap()
            .style
            .is_empty(),
        "window style list"
    );
    assert!(
        control_style(&project, "win", "ctl").is_empty(),
        "control style list"
    );
    assert_eq!(kinds(&changes), ["clear-style", "set-window"]);
}

#[test]
fn clearing_an_unknown_style_changes_nothing() {
    let mut changes = Vec::new();
    let result = project().clear_style(&mut changes, &style_id("nope"));

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn renaming_a_style_rewrites_every_list_that_names_it() {
    let project = project_using("old");

    let mut changes = Vec::new();
    let project = project
        .rename_style(&mut changes, &style_id("old"), style_id("new"))
        .unwrap()
        .expect("it was there");

    assert!(project.styles().contains(&style_id("new")));
    assert!(!project.styles().contains(&style_id("old")));
    assert_eq!(
        project
            .windows()
            .get_required(&window_id("win"))
            .unwrap()
            .style
            .as_slice(),
        [style_id("new")]
    );
    assert_eq!(
        control_style(&project, "win", "ctl").as_slice(),
        [style_id("new")]
    );
    assert_eq!(kinds(&changes), ["rename-style", "set-window"]);
}

#[test]
fn a_renamed_style_keeps_its_list_sorted() {
    let project = project()
        .set_style(&mut Vec::new(), style_id("a"), css())
        .set_style(&mut Vec::new(), style_id("m"), css())
        .new_window(&mut Vec::new(), window_id("win"))
        .unwrap()
        .set_window_properties(
            &mut Vec::new(),
            window_id("win"),
            WindowProperties {
                style: Patch::Set(StyleList::new(vec![style_id("a"), style_id("m")])),
                ..Default::default()
            },
        )
        .unwrap()
        .expect("the window changed");

    // 'a' becomes 'z', so the list has to be re-sorted rather than written back
    // in place.
    let project = project
        .rename_style(&mut Vec::new(), &style_id("a"), style_id("z"))
        .unwrap()
        .expect("it was there");

    assert_eq!(
        project
            .windows()
            .get_required(&window_id("win"))
            .unwrap()
            .style
            .as_slice(),
        [style_id("m"), style_id("z")]
    );
}

#[test]
fn renaming_a_style_to_its_own_id_changes_nothing() {
    let project = project_using("red");

    let mut changes = Vec::new();
    let result = project
        .rename_style(&mut changes, &style_id("red"), style_id("red"))
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn renaming_a_style_onto_a_taken_id_is_refused() {
    let project = project()
        .set_style(&mut Vec::new(), style_id("a"), css())
        .set_style(&mut Vec::new(), style_id("b"), css());

    let mut changes = Vec::new();
    let error = project
        .rename_style(&mut changes, &style_id("a"), style_id("b"))
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::AlreadyExists {
            kind: IdKind::Style,
            ..
        }
    ));
    assert!(changes.is_empty());
}

#[test]
fn renaming_an_unknown_style_is_refused() {
    let error = project()
        .rename_style(&mut Vec::new(), &style_id("nope"), style_id("new"))
        .unwrap_err();

    assert!(matches!(
        error,
        UiModelError::NotFound {
            kind: IdKind::Style,
            ..
        }
    ));
}
