//! The conversions between the model and the API types, and what a session is
//! told when it opens a project.

use studio_web_api::project_manager;

use super::super::model::wire;
use super::fixture::*;

#[test]
fn the_wire_conversion_is_a_fixed_point() {
    // The actor reads the project in and writes it back out on every call, so
    // anything the conversion loses or invents would show up as drift in the
    // stored project.
    let project = populated_project();

    let once = project_manager::CoreProject::from(&project);
    let twice = project_manager::CoreProject::from(&Project::try_from(&once).unwrap());

    assert_eq!(
        serde_json::to_value(&once).unwrap(),
        serde_json::to_value(&twice).unwrap()
    );
}

#[test]
fn the_outbound_conversion_carries_every_part_of_a_project() {
    // A fixed point only catches drift *between* the two passes, so the
    // outbound side is asserted on directly.
    let wire = project_manager::CoreProject::from(&populated_project());

    assert_eq!(wire.components.len(), 3, "usage, source and target");
    assert_eq!(wire.bindings.len(), 1);
    assert_eq!(wire.plugins.len(), 1);
    assert_eq!(wire.templates.len(), 1);

    let plugin = wire.plugins.values().next().expect("the plugin");
    assert!(
        matches!(
            plugin.toolbox_display,
            project_manager::CoreToolboxDisplay::Hide
        ),
        "the display survives"
    );

    let template = wire.templates.get("tpl").expect("the template");
    assert_eq!(template.components.len(), 1);
    assert!(template.exports.config.contains_key("label"));
    assert!(template.exports.members.contains_key("out"));

    let usage = wire.components.get("usage").expect("the component");
    assert!(matches!(
        usage.definition.r#type,
        project_manager::CoreComponentDefinitionType::Template
    ));
    assert_eq!(usage.definition.id, "tpl");
    assert!(
        usage
            .config
            .as_ref()
            .expect("a config")
            .0
            .contains_key("label"),
        "the exported config item reached the component using the template"
    );
}

#[test]
fn a_null_template_id_is_the_project_view() {
    // Every core call carries `templateId: string | null`. Resolving it once,
    // here, is what lets the model take a single `ViewKey`.
    assert_eq!(wire::view_key(None).unwrap(), ViewKey::Project);
    assert_eq!(wire::view_key(Some("")).unwrap(), ViewKey::Project);
    assert_eq!(wire::view_key(Some("tpl")).unwrap(), template_view("tpl"));
    assert!(
        wire::view_key(Some("a:b")).is_err(),
        "an id the client made up is refused at the boundary"
    );
}

#[test]
fn call_payload_ids_are_typed_at_the_boundary() {
    assert_eq!(
        wire::component_ids(&[String::from("a"), String::from("b")]).unwrap(),
        [component_id("a"), component_id("b")]
    );
    assert!(wire::component_ids(&[String::from("a:b")]).is_err());

    assert_eq!(
        wire::binding_id("a:state:b:action").unwrap().as_str(),
        "a:state:b:action"
    );
    assert!(wire::binding_id("").is_err());
}

#[test]
fn a_project_the_model_refuses_does_not_convert() {
    // The inbound conversion revalidates everything, so a stored project with
    // an id the model would not accept fails here rather than later.
    let mut wire = project_manager::CoreProject::from(&populated_project());
    let component = wire.components.values().next().unwrap().clone();
    wire.components.insert(String::from("bad:id"), component);

    assert!(matches!(
        Project::try_from(&wire),
        Err(CoreModelError::ForbiddenCharacter { .. })
    ));
}

#[test]
fn what_a_new_session_is_told_covers_the_whole_project() {
    let mut changes = Vec::new();
    populated_project().initial_changes(&mut changes);

    let emitted = kinds(&changes);

    assert_eq!(emitted[0], "set-plugins", "the plugin set comes first");
    assert_eq!(
        emitted
            .iter()
            .filter(|kind| **kind == "set-template")
            .count(),
        1
    );
    assert_eq!(
        emitted
            .iter()
            .filter(|kind| **kind == "set-component")
            .count(),
        4,
        "three on the project view, one inside the template"
    );
    assert_eq!(
        emitted
            .iter()
            .filter(|kind| **kind == "set-binding")
            .count(),
        1
    );
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
fn every_change_becomes_the_notification_that_matches_it() {
    use project_manager::UpdateProjectNotification as N;

    // One of each, built by the operations that emit them.
    let mut changes = vec![Change::Reset];
    let project = populated_project();
    project.initial_changes(&mut changes);

    project
        .set_plugin_display(&mut changes, &plugin_id(), ToolboxDisplay::Show)
        .unwrap()
        .expect("the display changed")
        .rename_component(
            &mut changes,
            &ViewKey::Project,
            &component_id("source"),
            component_id("renamed"),
        )
        .unwrap()
        .clear_components(&mut changes, &ViewKey::Project, &[component_id("target")])
        .unwrap();

    let notifications = wire::notifications(&changes);
    assert_eq!(notifications.len(), changes.len());

    for (change, notification) in changes.iter().zip(&notifications) {
        let matched = matches!(
            (kind(change), notification),
            ("reset", N::Reset(_))
                | ("set-plugins", N::SetCorePlugins(_))
                | ("set-plugin-display", N::SetCorePluginToolboxDisplay(_))
                | ("set-template", N::SetCoreTemplate(_))
                | ("set-component", N::SetCoreComponent(_))
                | ("clear-component", N::ClearCoreComponent(_))
                | ("rename-component", N::RenameCoreComponent(_))
                | ("set-binding", N::SetCoreBinding(_))
                | ("clear-binding", N::ClearCoreBinding(_))
        );

        assert!(matched, "{} did not convert as itself", kind(change));
    }
}

#[test]
fn a_change_carries_the_view_it_happened_in() {
    // The wire spells a view as a nullable `templateId`, so the conversion is
    // the only place that has to know about null.
    let mut changes = Vec::new();
    project_with_template()
        .set_component(
            &mut changes,
            &template_view("tpl"),
            component_id("{{id}}-another"),
            plugin_definition(),
            Position::default(),
        )
        .unwrap();

    let project_manager::UpdateProjectNotification::SetCoreComponent(notification) =
        &wire::notifications(&changes)[0]
    else {
        panic!("expected a component notification");
    };

    assert_eq!(notification.template_id.as_deref(), Some("tpl"));
}

/// The core designer sends `templateId: null` for anything outside a
/// template, so the call types have to accept it. Asserting it on `view_key`
/// alone is not enough: that never sees the JSON.
#[test]
fn the_client_payloads_with_a_null_template_deserialize() {
    use studio_web_api::project_manager as api;

    let call: api::SetComponentCoreProjectCall = serde_json::from_value(serde_json::json!({
        "templateId": null,
        "componentId": "comp",
        "definition": { "type": "plugin", "id": "inst:mod.plug" },
        "x": 1,
        "y": 2,
    }))
    .unwrap();
    assert_eq!(call.template_id, None);

    serde_json::from_value::<api::MoveComponentsCoreProjectCall>(serde_json::json!({
        "templateId": null,
        "componentsIds": ["comp"],
        "delta": { "x": 1, "y": 2 },
    }))
    .unwrap();

    serde_json::from_value::<api::ClearComponentsCoreProjectCall>(serde_json::json!({
        "templateId": null,
        "componentsIds": ["comp"],
    }))
    .unwrap();

    serde_json::from_value::<api::ClearBindingCoreProjectCall>(serde_json::json!({
        "templateId": null,
        "bindingId": "a:b:c:d",
    }))
    .unwrap();
}
