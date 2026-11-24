//! Element paths: how a reference says where it lives, and how that survives
//! the trip to the client and back.

use studio_web_api::project_manager;

use super::fixture::*;

fn wire_path(nodes: &[(&str, &str)]) -> project_manager::UiElementPath {
    nodes
        .iter()
        .map(|(kind, id)| {
            std::sync::Arc::new(project_manager::UiElementPathNode {
                r#type: String::from(*kind),
                id: String::from(*id),
            })
        })
        .collect()
}

#[test]
fn a_path_starts_at_the_view_it_lives_in() {
    let window: ElementPath = [PathNode::Window(window_id("win"))].into_iter().collect();
    let template: ElementPath = [PathNode::Template(template_id("tpl"))]
        .into_iter()
        .collect();

    assert_eq!(window.view_key().unwrap(), window_view("win"));
    assert_eq!(template.view_key().unwrap(), template_view("tpl"));
}

#[test]
fn a_path_that_starts_anywhere_else_has_no_view() {
    let path: ElementPath = [PathNode::Control(control_id("ctl"))].into_iter().collect();

    assert!(matches!(
        path.view_key(),
        Err(UiModelError::PathRootIsNotAView { node: "control" })
    ));
}

#[test]
fn an_empty_path_names_nothing() {
    let path = ElementPath::new();

    assert!(matches!(path.view_key(), Err(UiModelError::EmptyPath)));
    assert!(matches!(path.element(), Err(UiModelError::EmptyPath)));
}

#[test]
fn the_element_is_the_node_below_the_view() {
    let path: ElementPath = [
        PathNode::Window(window_id("win")),
        PathNode::Control(control_id("ctl")),
        PathNode::Display,
    ]
    .into_iter()
    .collect();

    assert_eq!(
        path.element().unwrap(),
        &PathNode::Control(control_id("ctl"))
    );
}

#[test]
fn every_node_survives_the_wire() {
    let path: ElementPath = [
        PathNode::Window(window_id("win")),
        PathNode::TemplateInstance(instance_id("inst")),
        PathNode::Binding(export_id("exp")),
        PathNode::Control(control_id("ctl")),
        PathNode::Display,
        PathNode::Text,
        PathNode::ContextItem(3),
        PathNode::PrimaryAction,
        PathNode::SecondaryAction,
        PathNode::Template(template_id("tpl")),
    ]
    .into_iter()
    .collect();

    let wire = project_manager::UiElementPath::from(&path);
    assert_eq!(ElementPath::try_from(&wire).unwrap(), path);
}

#[test]
fn a_node_type_the_model_does_not_know_is_refused() {
    // Parsing happens once, here, so no walk downstream needs an arm for the
    // unexpected.
    for nodes in [
        vec![("nonsense", "x")],
        vec![("control-type", "sideways")],
        vec![("action", "tertiary-action")],
        vec![("context-item", "not-a-number")],
    ] {
        assert!(
            matches!(
                ElementPath::try_from(&wire_path(&nodes)),
                Err(UiModelError::UnknownPathNode { .. })
            ),
            "{nodes:?} should be refused"
        );
    }
}

#[test]
fn a_node_whose_id_breaks_the_id_rules_is_refused() {
    assert!(matches!(
        ElementPath::try_from(&wire_path(&[("window", "a:b")])),
        Err(UiModelError::ForbiddenCharacter { .. })
    ));
}

#[test]
fn a_collected_usage_path_round_trips() {
    let project = populated_project();
    let usage = project.collect_component_usage();

    // The display reference, and the instance binding that points at the same
    // component member.
    assert_eq!(usage.len(), 2);

    for item in &usage {
        let wire = project_manager::UiElementPath::from(&item.path);
        assert_eq!(ElementPath::try_from(&wire).unwrap(), item.path);
    }

    let shapes: Vec<Vec<&str>> = usage
        .iter()
        .map(|item| item.path.nodes().iter().map(|node| node.kind()).collect())
        .collect();

    assert!(shapes.contains(&vec!["window", "control", "control-type"]));
    assert!(shapes.contains(&vec!["window", "template-instance", "template-binding"]));
}
