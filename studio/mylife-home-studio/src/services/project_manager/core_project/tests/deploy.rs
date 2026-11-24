//! Planning a deploy onto the running instances.

use std::collections::BTreeMap;

use studio_web_api::core_model::{BindingConfig, ComponentConfig};
use studio_web_api::project_manager::ChangeType;

use super::super::deploy::{
    OnlineStore, TaskChange, TaskObject, binding_config, component_config, prepare_to_online,
};
use super::super::resolver::{ResolvedProject, resolve};
use super::fixture::*;

fn resolved(project: &Project) -> ResolvedProject {
    resolve(project).expect("the project resolves")
}

/// Nothing running yet.
fn nothing_deployed() -> OnlineStore {
    OnlineStore {
        bindings_instance: Some(String::from("inst")),
        bindings: Vec::new(),
        components: BTreeMap::new(),
    }
}

fn running(components: Vec<ComponentConfig>, bindings: Vec<BindingConfig>) -> OnlineStore {
    OnlineStore {
        bindings_instance: Some(String::from("inst")),
        bindings,
        components: BTreeMap::from([(String::from("inst"), components)]),
    }
}

/// What a component of the fixtures' plugin looks like on the wire.
fn deployed(id: &str) -> ComponentConfig {
    ComponentConfig {
        id: String::from(id),
        plugin: String::from("module.name"),
        config: [
            (String::from("label"), serde_json::json!("")),
            (String::from("count"), serde_json::json!(0)),
        ]
        .into_iter()
        .collect(),
    }
}

fn tasks(plan: &super::super::deploy::OnlinePlan) -> Vec<String> {
    plan.tasks
        .iter()
        .map(|task| {
            let what = match &task.object {
                TaskObject::Component { id } => format!("component {id}"),
                TaskObject::Binding { id } => format!("binding {id}"),
            };
            let change = match task.change_type {
                TaskChange::Add => "add",
                TaskChange::Delete => "delete",
            };
            format!("{change} {what}")
        })
        .collect()
}

#[test]
fn deploying_onto_an_empty_instance_adds_everything() {
    let project = resolved(&project_with_binding());
    let plan = prepare_to_online(&project, &nothing_deployed()).unwrap();

    assert_eq!(
        tasks(&plan),
        [
            "add component source",
            "add component target",
            "add binding source:state:target:action"
        ],
        "components first: a binding cannot be made before its ends exist"
    );

    assert_eq!(plan.changes.components.len(), 2);
    assert_eq!(plan.changes.bindings.len(), 1);
    assert!(matches!(plan.changes.components[0].r#type, ChangeType::Add));
}

#[test]
fn a_component_already_running_unchanged_is_left_alone() {
    let project = resolved(&project_with_components());
    let store = running(vec![deployed("source"), deployed("target")], Vec::new());

    let plan = prepare_to_online(&project, &store).unwrap();

    assert!(tasks(&plan).is_empty());
    assert!(plan.changes.components.is_empty());
}

#[test]
fn a_component_whose_config_changed_is_removed_and_added_again() {
    // There is no update on the wire, so a change is two tasks.
    let project = resolved(&project_with_components());

    let mut stale = deployed("source");
    stale
        .config
        .insert(String::from("label"), serde_json::json!("old"));

    let plan = prepare_to_online(
        &project,
        &running(vec![stale, deployed("target")], Vec::new()),
    )
    .unwrap();

    assert_eq!(
        tasks(&plan),
        ["delete component source", "add component source"]
    );
    assert_eq!(plan.changes.components.len(), 1);
    assert!(matches!(
        plan.changes.components[0].r#type,
        ChangeType::Update
    ));
}

#[test]
fn a_component_whose_plugin_changed_is_removed_and_added_again() {
    let project = resolved(&project_with_components());

    let mut stale = deployed("source");
    stale.plugin = String::from("module.other");

    let plan = prepare_to_online(
        &project,
        &running(vec![stale, deployed("target")], Vec::new()),
    )
    .unwrap();

    assert_eq!(
        tasks(&plan),
        ["delete component source", "add component source"]
    );
}

#[test]
fn anything_running_that_the_project_does_not_have_is_removed() {
    // An instance is deployed whole from one project, so a leftover goes.
    let project = resolved(&project_with_components());
    let store = running(
        vec![deployed("source"), deployed("target"), deployed("stale")],
        Vec::new(),
    );

    let plan = prepare_to_online(&project, &store).unwrap();

    assert_eq!(tasks(&plan), ["delete component stale"]);
    assert!(matches!(
        plan.changes.components[0].r#type,
        ChangeType::Delete
    ));
}

#[test]
fn bindings_come_off_first_and_go_on_last() {
    // A binding cannot outlive either of the components it joins, so the
    // removals lead and the additions trail.
    let project = resolved(&project_with_binding());

    let stale_binding = BindingConfig {
        source_component: String::from("gone"),
        source_state: String::from("state"),
        target_component: String::from("also-gone"),
        target_action: String::from("action"),
    };

    let store = running(vec![deployed("stale")], vec![stale_binding]);
    let plan = prepare_to_online(&project, &store).unwrap();

    assert_eq!(
        tasks(&plan),
        [
            "delete binding gone:state:also-gone:action",
            "delete component stale",
            "add component source",
            "add component target",
            "add binding source:state:target:action",
        ]
    );
}

#[test]
fn a_binding_already_running_is_left_alone() {
    let project = resolved(&project_with_binding());

    let store = running(
        vec![deployed("source"), deployed("target")],
        vec![BindingConfig {
            source_component: String::from("source"),
            source_state: String::from("state"),
            target_component: String::from("target"),
            target_action: String::from("action"),
        }],
    );

    assert!(tasks(&prepare_to_online(&project, &store).unwrap()).is_empty());
}

#[test]
fn a_component_inside_a_template_deploys_under_its_folded_id() {
    let project = resolved(&populated_project());
    let plan = prepare_to_online(&project, &nothing_deployed()).unwrap();

    assert!(
        tasks(&plan).contains(&String::from("add component usage-inner")),
        "{:?}",
        tasks(&plan)
    );
}

#[test]
fn a_project_with_no_bindings_needs_no_instance_to_carry_them() {
    let project = resolved(&project_with_components());

    let store = OnlineStore {
        bindings_instance: None,
        bindings: Vec::new(),
        components: BTreeMap::new(),
    };

    assert!(prepare_to_online(&project, &store).is_ok());
}

#[test]
fn bindings_with_nowhere_to_run_are_refused() {
    let project = resolved(&project_with_binding());

    let store = OnlineStore {
        bindings_instance: None,
        bindings: Vec::new(),
        components: BTreeMap::new(),
    };

    assert!(matches!(
        prepare_to_online(&project, &store),
        Err(CoreModelError::BindingsInstanceNeeded)
    ));
}

#[test]
fn a_component_is_described_by_the_plugin_it_runs() {
    let project = resolved(&project_with_components());
    let config = component_config(&project, &component_id("source")).unwrap();

    assert_eq!(config.id, "source");
    assert_eq!(
        config.plugin, "module.name",
        "the instance is not part of it: the instance is where it is sent"
    );
    assert_eq!(config.config.get("count"), Some(&serde_json::json!(0)));
}

#[test]
fn a_binding_id_goes_back_into_its_four_parts() {
    // A binding only running online is known by its id alone, so the split
    // has to be exact.
    let id = binding("source", "state", "target", "action").id().unwrap();
    let config = binding_config(&id).unwrap();

    assert_eq!(config.source_component, "source");
    assert_eq!(config.source_state, "state");
    assert_eq!(config.target_component, "target");
    assert_eq!(config.target_action, "action");

    assert!(binding_config(&BindingId::new("not-four-parts").unwrap()).is_err());
}
