//! Flattening a project: templates expanded, ids folded, exports resolved.

use super::super::resolver::{ResolvedProject, resolve};
use super::fixture::*;

fn resolved(project: &Project) -> ResolvedProject {
    resolve(project).expect("the project resolves")
}

fn component_ids(resolved: &ResolvedProject) -> Vec<String> {
    resolved
        .components()
        .keys()
        .map(ToString::to_string)
        .collect()
}

fn binding_ids(resolved: &ResolvedProject) -> Vec<String> {
    resolved
        .bindings()
        .keys()
        .map(ToString::to_string)
        .collect()
}

fn config_of(resolved: &ResolvedProject, id: &str, name: &str) -> Option<String> {
    resolved
        .component(&component_id(id))
        .unwrap()
        .config
        .get(name)
        .map(ToString::to_string)
}

/// `tpl` gains a second component and a binding between the two, so there is
/// something inside the template to resolve.
fn template_with_a_binding() -> Project {
    project_with_template()
        .set_component(
            &mut Vec::new(),
            &template_view("tpl"),
            component_id("{{id}}-other"),
            plugin_definition(),
            Position::default(),
        )
        .unwrap()
        .set_binding(
            &mut Vec::new(),
            &template_view("tpl"),
            binding("{{id}}-inner", "state", "{{id}}-other", "action"),
        )
        .unwrap()
}

#[test]
fn a_project_without_templates_resolves_to_itself() {
    let resolved = resolved(&project_with_binding());

    assert_eq!(component_ids(&resolved), ["source", "target"]);
    assert_eq!(binding_ids(&resolved), ["source:state:target:action"]);
    assert_eq!(
        resolved.plugins().len(),
        1,
        "plugins are carried as they are"
    );
}

#[test]
fn a_template_component_becomes_the_components_inside_it() {
    // The component that uses the template does not survive: at deploy time
    // there is no such thing, only what the template expanded to.
    let resolved = resolved(&project_with_template());

    assert_eq!(component_ids(&resolved), ["usage-inner"]);
    assert_eq!(
        resolved
            .component(&component_id("usage-inner"))
            .unwrap()
            .plugin,
        plugin_id()
    );
}

#[test]
fn the_placeholder_is_filled_with_the_id_of_whatever_used_the_template() {
    let project = project_with_template()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("second"),
            template_definition("tpl"),
            Position::default(),
        )
        .unwrap();

    assert_eq!(
        component_ids(&resolved(&project)),
        ["second-inner", "usage-inner"],
        "one set per use, and they do not collide"
    );
}

#[test]
fn an_id_folds_through_every_level_it_passes() {
    let project = project_with_template()
        .set_template(&mut Vec::new(), template_id("outer"))
        .unwrap()
        .set_component(
            &mut Vec::new(),
            &template_view("outer"),
            component_id("{{id}}-mid"),
            template_definition("tpl"),
            Position::default(),
        )
        .unwrap()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("top"),
            template_definition("outer"),
            Position::default(),
        )
        .unwrap();

    let ids = component_ids(&resolved(&project));

    assert!(ids.contains(&String::from("top-mid-inner")), "{ids:?}");
    assert!(ids.contains(&String::from("usage-inner")), "{ids:?}");
}

#[test]
fn an_exported_config_item_is_supplied_by_whoever_used_the_template() {
    // The inner component stopped carrying `label` when it was exported; the
    // value now comes from the component that instantiates the template.
    let project = populated_project()
        .configure_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("usage"),
            "label",
            serde_json::json!("from the outside"),
        )
        .unwrap();

    let resolved = resolved(&project);

    assert_eq!(
        config_of(&resolved, "usage-inner", "label"),
        Some(String::from("\"from the outside\""))
    );
}

#[test]
fn a_config_item_nothing_exports_keeps_the_value_it_had_inside() {
    let project = populated_project()
        .configure_component(
            &mut Vec::new(),
            &template_view("tpl"),
            component_id("{{id}}-inner"),
            "count",
            serde_json::json!(7),
        )
        .unwrap();

    let resolved = resolved(&project);

    assert_eq!(
        config_of(&resolved, "usage-inner", "count"),
        Some(String::from("7"))
    );
}

#[test]
fn a_binding_inside_a_template_is_resolved_between_the_expanded_components() {
    let resolved = resolved(&template_with_a_binding());

    assert_eq!(
        binding_ids(&resolved),
        ["usage-inner:state:usage-other:action"],
        "both ends folded the same way the components did"
    );
}

#[test]
fn a_template_used_twice_gets_its_internal_bindings_twice() {
    let project = template_with_a_binding()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("second"),
            template_definition("tpl"),
            Position::default(),
        )
        .unwrap();

    assert_eq!(
        binding_ids(&resolved(&project)),
        [
            "second-inner:state:second-other:action",
            "usage-inner:state:usage-other:action"
        ]
    );
}

#[test]
fn a_binding_onto_a_member_export_reaches_the_component_behind_it() {
    // `out` is an export of `tpl` pointing at the inner component's `state`.
    // The resolved binding names the inner component, not the template user.
    let project = populated_project()
        .set_binding(
            &mut Vec::new(),
            &ViewKey::Project,
            binding("usage", "out", "target", "action"),
        )
        .unwrap();

    let ids = binding_ids(&resolved(&project));

    assert!(
        ids.contains(&String::from("usage-inner:state:target:action")),
        "{ids:?}"
    );
}

#[test]
fn a_member_export_resolves_through_a_nested_template() {
    // `outer` exports a member that points at a component which is itself a
    // template, so the walk has to go down twice.
    let project = populated_project()
        .set_template(&mut Vec::new(), template_id("outer"))
        .unwrap()
        .set_component(
            &mut Vec::new(),
            &template_view("outer"),
            component_id("{{id}}-mid"),
            template_definition("tpl"),
            Position::default(),
        )
        .unwrap()
        .set_template_export(
            &mut Vec::new(),
            &template_id("outer"),
            ExportKind::Member,
            export_id("deep"),
            &component_id("{{id}}-mid"),
            "out",
        )
        .unwrap()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("top"),
            template_definition("outer"),
            Position::default(),
        )
        .unwrap()
        .set_binding(
            &mut Vec::new(),
            &ViewKey::Project,
            binding("top", "deep", "target", "action"),
        )
        .unwrap();

    let ids = binding_ids(&resolved(&project));

    assert!(
        ids.contains(&String::from("top-mid-inner:state:target:action")),
        "{ids:?}"
    );
}

#[test]
fn an_external_component_stays_external_and_a_template_child_does_not() {
    let resolved = resolved(&populated_project());

    assert!(
        !resolved
            .component(&component_id("usage-inner"))
            .unwrap()
            .external
    );
    assert!(
        !resolved
            .component(&component_id("source"))
            .unwrap()
            .external
    );
}

#[test]
fn instances_name_only_the_components_this_project_deploys() {
    let instances = resolved(&populated_project()).instances().unwrap();

    assert_eq!(
        instances
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["inst"]
    );
    assert_eq!(
        instances[&instance_id("inst")]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["source", "target", "usage-inner"]
    );
}
