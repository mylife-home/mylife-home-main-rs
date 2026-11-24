//! The deploy pass: flattening a project into the definition the runtime UI
//! consumes, and reporting every reason it cannot be.

use std::collections::HashMap;

use studio_web_api::ui_model;

use super::super::deploy::deploy;
use super::fixture::*;

fn definition(project: &Project) -> ui_model::Definition {
    match deploy(project) {
        Ok(definition) => definition,
        Err(errors) => panic!("expected a deployable project, got {errors:?}"),
    }
}

/// The messages a failed deploy reported, sorted so a test does not depend on
/// the order two independent errors happen to come out in.
fn errors(project: &Project) -> Vec<String> {
    let mut messages: Vec<_> = deploy(project)
        .err()
        .expect("expected the deploy to be refused")
        .into_iter()
        .map(|error| error.message)
        .collect();

    messages.sort();
    messages
}

fn window<'a>(definition: &'a ui_model::Definition, id: &str) -> &'a ui_model::Window {
    definition
        .windows
        .iter()
        .find(|window| window.id == id)
        .expect("the window")
}

fn control<'a>(window: &'a ui_model::Window, id: &str) -> &'a ui_model::Control {
    window
        .controls
        .iter()
        .find(|control| control.id == id)
        .expect("the control")
}

/// `tpl` gains a control of its own, so an instance of it has something to
/// flatten into its host window.
fn template_with_control() -> Project {
    let project = populated_project()
        .new_control(
            &mut Vec::new(),
            &template_view("tpl"),
            control_id("inner"),
            5,
            6,
            ControlKind::Display,
        )
        .unwrap();

    set_control(
        &project,
        &template_view("tpl"),
        &control_id("inner"),
        display_properties(Some(MemberRef::OwnerExport(export_id("exp")))),
    )
}

// ---------------------------------------------------------------------------
// What a deploy produces
// ---------------------------------------------------------------------------

#[test]
fn an_empty_project_deploys_to_an_empty_definition() {
    let definition = definition(&project());

    assert!(definition.windows.is_empty());
    assert!(definition.resources.is_empty());
    assert!(definition.styles.is_empty());
    assert!(definition.default_window.0.is_empty());
}

#[test]
fn a_project_deploys_with_its_resources_styles_and_default_windows() {
    let definition = definition(&populated_project());

    assert_eq!(definition.resources.len(), 1);
    assert_eq!(definition.resources[0].id, "bg");
    assert_eq!(definition.resources[0].mime, "image/png");

    assert_eq!(definition.styles.len(), 1);
    assert_eq!(definition.styles[0].id, "big");

    assert_eq!(
        definition.default_window.0.get("desktop"),
        Some(&String::from("win")),
        "a context pointing at a window is carried"
    );
}

#[test]
fn a_control_deploys_with_the_component_it_points_at() {
    let definition = definition(&populated_project());
    let control = control(window(&definition, "win"), "ctl");

    let display = control.display.as_ref().expect("a display");
    assert_eq!(display.component_id.as_deref(), Some("comp"));
    assert_eq!(display.component_state.as_deref(), Some("state"));
    assert_eq!((control.x, control.y), (10, 20));
}

#[test]
fn a_window_carries_its_style_and_background() {
    let definition = definition(&populated_project());
    let window = window(&definition, "win");

    assert_eq!(window.style.0, vec![String::from("big")]);
    assert_eq!(window.background_resource.as_ref().unwrap().0, "bg");
}

#[test]
fn an_instance_is_flattened_into_the_window_that_uses_it() {
    // The template itself does not reach the runtime: its controls do, once per
    // instance, with the instance id prefixed and the position made absolute.
    let definition = definition(&template_with_control());
    let window = window(&definition, "win");

    assert_eq!(
        window.controls.len(),
        2,
        "its own control, and the inner one"
    );

    let inner = control(window, "inst:inner");
    assert_eq!(
        (inner.x, inner.y),
        (35, 46),
        "the instance is at (30, 40) and the control at (5, 6)"
    );
}

#[test]
fn a_reference_to_an_enclosing_export_resolves_through_the_binding() {
    // `inner` points at `exp`, and the instance binds `exp` to comp/state. The
    // deployed control points at the component, with no template left.
    let definition = definition(&template_with_control());
    let inner = control(window(&definition, "win"), "inst:inner");

    let display = inner.display.as_ref().expect("a display");
    assert_eq!(display.component_id.as_deref(), Some("comp"));
    assert_eq!(display.component_state.as_deref(), Some("state"));
}

#[test]
fn the_same_template_used_twice_deploys_twice_under_different_ids() {
    let project = template_with_control()
        .new_template_instance(
            &mut Vec::new(),
            &window_view("win"),
            instance_id("second"),
            template_id("tpl"),
            100,
            100,
        )
        .unwrap()
        .set_template_instance_bindings(
            &mut Vec::new(),
            &window_view("win"),
            instance_id("second"),
            HashMap::from([(export_id("exp"), Some(component_ref("comp", "state")))]),
        )
        .unwrap();

    let definition = definition(&project);
    let window = window(&definition, "win");

    assert_eq!(window.controls.len(), 3);
    assert_eq!(
        (
            control(window, "second:inner").x,
            control(window, "second:inner").y
        ),
        (105, 106)
    );
}

// ---------------------------------------------------------------------------
// What a deploy refuses
// ---------------------------------------------------------------------------

#[test]
fn a_control_that_is_neither_image_nor_text_is_refused() {
    let project = populated_project();
    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        ControlProperties {
            display: Patch::Set(None),
            ..Default::default()
        },
    );

    assert!(
        errors(&project).contains(&String::from("Le contrôle doit être image ou texte")),
        "neither half is set"
    );
}

#[test]
fn a_display_with_a_map_has_to_say_what_it_is_mapping() {
    // Without a map an unbound display is a blank image, which is allowed. With
    // one there is nothing to map.
    let project = populated_project();

    assert!(
        deploy(&project).is_ok(),
        "an unbound display with no map is deployable"
    );

    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        ControlProperties {
            display: Patch::Set(Some(std::sync::Arc::new(ControlDisplay {
                member: None,
                default_resource: None,
                map: vec![ControlDisplayMapItem {
                    min: None,
                    max: None,
                    value: serde_json::Value::Bool(true),
                    resource: Some(resource_id("bg")),
                }],
            }))),
            ..Default::default()
        },
    );

    assert!(errors(&project).contains(&String::from("Le composant n'est pas défini.")));
}

#[test]
fn a_map_entry_has_to_match_the_type_it_maps() {
    let project = populated_project();

    // `state` is a bool, so a min/max entry means nothing.
    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        ControlProperties {
            display: Patch::Set(Some(std::sync::Arc::new(ControlDisplay {
                member: Some(component_ref("comp", "state")),
                default_resource: None,
                map: vec![ControlDisplayMapItem {
                    min: Some(0),
                    max: Some(10),
                    value: serde_json::Value::Null,
                    resource: Some(resource_id("bg")),
                }],
            }))),
            ..Default::default()
        },
    );

    let errors = errors(&project);
    assert!(errors.contains(&String::from("Le type 'enum' ne doit pas utiliser min/max")));
    assert!(errors.contains(&String::from("Valeur incorrecte")));
}

#[test]
fn a_resource_that_does_not_exist_is_refused() {
    let project = populated_project();
    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        ControlProperties {
            display: Patch::Set(Some(std::sync::Arc::new(ControlDisplay {
                member: None,
                default_resource: Some(resource_id("gone")),
                map: Vec::new(),
            }))),
            ..Default::default()
        },
    );

    assert!(errors(&project).contains(&String::from("La ressource 'gone' n'existe pas.")));
}

#[test]
fn an_action_has_to_point_at_something() {
    // Deleting a window unsets the actions that opened it, which is the state
    // this reports.
    let project = populated_project()
        .new_window(&mut Vec::new(), window_id("target"))
        .unwrap();

    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        ControlProperties {
            primary_action: Patch::Set(Some(window_action(Some(window_id("target"))))),
            ..Default::default()
        },
    );

    assert!(deploy(&project).is_ok(), "while the window is there");

    let project = project
        .clear_window(&mut Vec::new(), &window_id("target"))
        .expect("it was there");

    assert!(errors(&project).contains(&String::from("L'identifiant n'est pas défini.")));
}

#[test]
fn an_action_has_to_point_at_a_bool() {
    // An action is triggered, so whatever it points at has to accept a bool.
    let project = populated_project();
    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        ControlProperties {
            primary_action: Patch::Set(Some(component_action(Some(component_ref(
                "comp", "level",
            ))))),
            ..Default::default()
        },
    );

    assert!(
        errors(&project)
            .iter()
            .any(|message| message.contains("n'existe pas"))
    );
}

#[test]
fn a_default_window_has_to_name_a_window_that_exists() {
    let project = project().set_default_window(
        &mut Vec::new(),
        DefaultWindow::new(HashMap::from([(
            String::from("desktop"),
            Some(window_id("gone")),
        )])),
    );

    assert!(errors(&project).contains(&String::from("La fenêtre 'gone' n'existe pas.")));
}

#[test]
fn a_context_without_a_window_is_refused() {
    let project = project().set_default_window(
        &mut Vec::new(),
        DefaultWindow::new(HashMap::from([(String::from("desktop"), None)])),
    );

    assert!(errors(&project).contains(&String::from("L'identifiant n'est pas défini.")));
}

#[test]
fn a_template_that_contains_itself_is_refused() {
    let project = project_with_template()
        .new_template_instance(
            &mut Vec::new(),
            &template_view("tpl"),
            instance_id("self"),
            template_id("tpl"),
            0,
            0,
        )
        .unwrap();

    assert!(errors(&project).contains(&String::from("Le template 'tpl' est récursif.")));
}

#[test]
fn a_binding_keyed_by_an_export_that_is_gone_is_refused() {
    // Clearing an export only reaches the instances inside the template that
    // owns it. An instance in another view keeps the slot, and this is where
    // that surfaces.
    let project = populated_project()
        .clear_template_export(&mut Vec::new(), template_id("tpl"), &export_id("exp"))
        .unwrap()
        .expect("it was there");

    assert!(errors(&project).contains(&String::from("Binding sur export 'exp' inexistant")));
}

#[test]
fn an_export_added_after_an_instance_leaves_it_with_nothing_to_bind() {
    let project = populated_project()
        .set_template_export(
            &mut Vec::new(),
            template_id("tpl"),
            export_id("added"),
            MemberType::State,
            "bool",
        )
        .unwrap();

    assert!(errors(&project).contains(&String::from("Binding inexistant sur export 'added'")));
}

#[test]
fn a_template_nothing_uses_is_still_checked() {
    // A template reaches the runtime only through the windows that use it, so
    // an unused one has nothing to deploy. It is still checked, or a template
    // would be reported as broken only once someone used it.
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
        display_properties(Some(component_ref("gone", "state"))),
    );

    assert!(errors(&project).contains(&String::from("Le composant 'gone' n'existe pas.")));
}

#[test]
fn every_problem_is_reported_in_one_pass() {
    // One error per attempt would mean one deploy per problem.
    let project = populated_project().set_default_window(
        &mut Vec::new(),
        DefaultWindow::new(HashMap::from([(
            String::from("desktop"),
            Some(window_id("gone")),
        )])),
    );

    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        ControlProperties {
            display: Patch::Set(None),
            primary_action: Patch::Set(Some(component_action(Some(component_ref(
                "gone", "action",
            ))))),
            ..Default::default()
        },
    );

    let errors = errors(&project);

    assert!(errors.contains(&String::from("La fenêtre 'gone' n'existe pas.")));
    assert!(errors.contains(&String::from("Le contrôle doit être image ou texte")));
    assert!(errors.contains(&String::from("Le composant 'gone' n'existe pas.")));
}

#[test]
fn an_error_says_where_it_was_found() {
    let project = populated_project();
    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        display_properties(Some(component_ref("gone", "state"))),
    );

    let errors = deploy(&project).err().expect("refused");
    let error = errors
        .iter()
        .find(|error| error.message.contains("'gone'"))
        .expect("the component error");

    let path: Vec<_> = error
        .path
        .iter()
        .map(|node| (node.r#type.as_str(), node.id.as_str()))
        .collect();

    assert_eq!(
        path,
        [
            ("window", "win"),
            ("control", "ctl"),
            ("control-type", "display")
        ]
    );
}

#[test]
fn two_broken_controls_in_one_view_are_both_reported() {
    // Every entry of a collection is visited even after one of them fails, so
    // a view with two problems takes one deploy to find out about both, not
    // two.
    let project = populated_project()
        .new_control(
            &mut Vec::new(),
            &window_view("win"),
            control_id("second"),
            0,
            0,
            ControlKind::Display,
        )
        .unwrap();

    let broken = ControlProperties {
        display: Patch::Set(None),
        ..Default::default()
    };

    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        broken.clone(),
    );
    let project = set_control(&project, &window_view("win"), &control_id("second"), broken);

    let reported: Vec<_> = deploy(&project)
        .err()
        .expect("refused")
        .iter()
        .filter_map(|error| {
            error
                .path
                .iter()
                .find(|node| node.r#type == "control")
                .map(|node| node.id.clone())
        })
        .collect();

    assert!(reported.contains(&String::from("ctl")));
    assert!(reported.contains(&String::from("second")));
}
