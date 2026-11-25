//! The conversions between the model and the API types: the one place the two
//! shapes meet.

use std::collections::HashMap;

use studio_web_api::project_manager;

use super::super::model::wire;
use super::fixture::*;

#[test]
fn the_wire_conversion_is_a_fixed_point() {
    // The actor reads the project in and writes it back out on every call, so
    // anything the conversion loses or invents would show up as drift in the
    // stored project. Comparing `Value`s rather than text keeps this
    // independent of map ordering.
    let project = populated_project();

    let once = project_manager::UiProject::from(&project);
    let twice = project_manager::UiProject::from(&Project::try_from(&once).unwrap());

    assert_eq!(
        serde_json::to_value(&once).unwrap(),
        serde_json::to_value(&twice).unwrap()
    );
}

#[test]
fn the_outbound_conversion_carries_every_part_of_a_window() {
    // A fixed point only catches drift *between* the two passes: anything the
    // outbound conversion drops is equally absent from both and still compares
    // equal. So the outbound side is asserted on directly.
    let wire = project_manager::UiProject::from(&populated_project());

    let window = wire.windows.get("win").expect("the window");

    assert_eq!(window.style.0, vec![String::from("big")]);
    assert_eq!(window.background_resource.as_ref().unwrap().0, "bg");
    assert_eq!(window.controls.len(), 1);
    assert_eq!(window.templates.len(), 1);
    assert_eq!(
        wire.default_window.0.get("desktop"),
        Some(&Some(String::from("win")))
    );
    assert!(wire.styles.contains_key("big"));
    assert!(wire.resources.contains_key("bg"));
    assert!(wire.templates.contains_key("tpl"));

    let binding = window
        .templates
        .get("inst")
        .expect("the instance")
        .bindings
        .get("exp")
        .expect("the binding");

    assert_eq!(binding.component_id.as_deref(), Some("comp"));
    assert_eq!(binding.member_name.as_deref(), Some("state"));
}

#[test]
fn a_context_with_no_window_survives_as_an_empty_slot() {
    // The designer can name a context before deciding what it shows, so the
    // slot and its absence are different things on the wire too.
    let project = project().set_default_window(
        &mut Vec::new(),
        DefaultWindow::new(HashMap::from([(String::from("mobile"), None)])),
    );

    let wire = project_manager::UiProject::from(&project);
    assert_eq!(wire.default_window.0.get("mobile"), Some(&None));

    let back = Project::try_from(&wire).unwrap();
    assert_eq!(
        back.default_window()
            .iter()
            .map(|(key, window)| (key.clone(), window.clone()))
            .collect::<Vec<_>>(),
        [(String::from("mobile"), None)]
    );
}

#[test]
fn a_reference_survives_the_wire_in_both_forms() {
    // A component reference is a pair of strings on the wire; an owner-export
    // reference is the same pair with the first half missing.
    let project = project_with_template()
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

    let round_tripped = Project::try_from(&project_manager::UiProject::from(&project)).unwrap();

    let member = round_tripped
        .templates()
        .get_required(&template_id("tpl"))
        .unwrap()
        .controls
        .get_required(&control_id("ctl"))
        .unwrap()
        .display
        .as_ref()
        .unwrap()
        .member
        .clone();

    assert_eq!(member, Some(MemberRef::OwnerExport(export_id("exp"))));

    let component = Project::try_from(&project_manager::UiProject::from(&populated_project()))
        .unwrap()
        .windows()
        .get_required(&window_id("win"))
        .unwrap()
        .controls
        .get_required(&control_id("ctl"))
        .unwrap()
        .display
        .as_ref()
        .unwrap()
        .member
        .clone();

    assert_eq!(component, Some(component_ref("comp", "state")));
}

#[test]
fn a_component_without_a_member_is_rejected_at_the_boundary() {
    // The pair the wire format allows and no operation can produce.
    let wire = HashMap::from([(
        String::from("exp"),
        project_manager::UiTemplateInstanceBinding {
            component_id: Some(String::from("comp")),
            member_name: None,
        },
    )]);

    assert_eq!(
        wire::bindings(wire).unwrap_err(),
        UiModelError::ComponentWithoutMember {
            context: "template instance binding",
            component: String::from("comp"),
        }
    );
}

#[test]
fn a_member_with_no_component_is_an_owner_export() {
    let wire = HashMap::from([(
        String::from("exp"),
        project_manager::UiTemplateInstanceBinding {
            component_id: None,
            member_name: Some(String::from("other")),
        },
    )]);

    assert_eq!(
        wire::bindings(wire).unwrap(),
        HashMap::from([(
            export_id("exp"),
            Some(MemberRef::OwnerExport(export_id("other")))
        )])
    );
}

#[test]
fn a_view_type_and_an_id_become_one_key() {
    assert_eq!(
        wire::view_key(project_manager::ViewType::Template, "tpl").unwrap(),
        template_view("tpl")
    );
    assert_eq!(
        wire::view_key(project_manager::ViewType::Window, "win").unwrap(),
        window_view("win")
    );
    assert!(
        wire::view_key(project_manager::ViewType::Window, "a:b").is_err(),
        "an id the client made up is refused at the boundary"
    );
}

#[test]
fn bulk_patterns_are_keyed_by_export_id_on_the_way_in() {
    assert_eq!(
        wire::bulk_patterns(HashMap::from([(
            String::from("exp"),
            String::from("{{id}}")
        )]))
        .unwrap(),
        HashMap::from([(export_id("exp"), String::from("{{id}}"))])
    );

    assert!(wire::bulk_patterns(HashMap::from([(String::new(), String::new())])).is_err());
}

#[test]
fn a_project_the_model_refuses_does_not_convert() {
    // The inbound conversion revalidates everything, so a stored project with
    // an id the model would not accept fails here rather than at deploy time.
    let mut wire = project_manager::UiProject::from(&populated_project());
    let window = wire.windows.values().next().unwrap().clone();

    let mut windows = wire.windows.as_ref().clone();
    windows.insert(String::from("bad:id"), window);
    wire.windows = std::sync::Arc::new(windows);

    assert!(matches!(
        Project::try_from(&wire),
        Err(UiModelError::ForbiddenCharacter { .. })
    ));
}

#[test]
fn every_change_becomes_the_notification_that_matches_it() {
    use project_manager::UpdateProjectNotification as N;

    let mut changes = Vec::new();
    let project = populated_project();
    project.initial_changes(&mut changes);

    let notifications = wire::notifications(&changes);
    assert_eq!(notifications.len(), changes.len());

    for (change, notification) in changes.iter().zip(&notifications) {
        let matched = matches!(
            (kind(change), notification),
            ("reset", N::Reset(_))
                | ("set-default-window", N::SetUiDefaultWindow(_))
                | ("set-component-data", N::SetUiComponentData(_))
                | ("set-resource", N::SetUiResource(_))
                | ("set-style", N::SetUiStyle(_))
                | ("set-window", N::SetUiWindow(_))
                | ("set-template", N::SetUiTemplate(_))
        );

        assert!(matched, "{} did not convert as itself", kind(change));
    }
}

#[test]
fn a_reset_is_a_change_like_any_other() {
    // A reload announces the project being replaced and the project that
    // replaced it as one list, so there is one path from changes to
    // notifications rather than two.
    let mut changes = vec![Change::Reset];
    populated_project().initial_changes(&mut changes);

    assert_eq!(kind(&changes[0]), "reset");

    let notifications = wire::notifications(&changes);
    assert_eq!(notifications.len(), changes.len());
    assert!(matches!(
        notifications[0],
        project_manager::UpdateProjectNotification::Reset(_)
    ));
}

#[test]
fn what_a_new_session_is_told_covers_the_whole_project() {
    let mut changes = Vec::new();
    populated_project().initial_changes(&mut changes);

    let mut emitted = kinds(&changes);
    emitted.sort_unstable();

    assert_eq!(
        emitted,
        [
            "set-component-data",
            "set-default-window",
            "set-resource",
            "set-style",
            "set-template",
            "set-window",
        ]
    );
}
