//! Component references: finding them, clearing them, and the refresh that
//! swaps the component set underneath them.

use std::sync::Arc;

use super::super::component::{self, NewComponentData, Refresh};
use super::fixture::*;

/// A control whose display and primary action both point at the same
/// component. Two references, one control, one view.
fn project_with_two_usages() -> Project {
    let project = project_with_view_contents();

    set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        ControlProperties {
            display: Patch::Set(Some(display_with(Some(component_ref("comp", "state"))))),
            primary_action: Patch::Set(Some(component_action(Some(component_ref(
                "comp", "action",
            ))))),
            ..Default::default()
        },
    )
}

fn display_member(project: &Project, view: &ViewKey, control: &str) -> Option<MemberRef> {
    project
        .view(view)
        .unwrap()
        .controls()
        .get_required(&control_id(control))
        .unwrap()
        .display
        .as_ref()
        .expect("a display control")
        .member
        .clone()
}

// ---------------------------------------------------------------------------
// Finding usage
// ---------------------------------------------------------------------------

#[test]
fn a_usage_names_the_exact_reference_it_came_from() {
    let project = project_with_two_usages();

    let usage = project.collect_component_usage();
    assert_eq!(usage.len(), 2);

    assert_eq!(
        usage[0].path.nodes()[..2],
        usage[1].path.nodes()[..2],
        "both are in the same view and control"
    );
    assert_ne!(
        usage[0].path, usage[1].path,
        "but they name different references inside it"
    );

    let last: Vec<_> = usage
        .iter()
        .map(|item| item.path.nodes().last().unwrap().clone())
        .collect();
    assert!(last.contains(&PathNode::Display));
    assert!(last.contains(&PathNode::PrimaryAction));
}

#[test]
fn every_kind_of_reference_is_collected() {
    let project = project_with_view_contents();

    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        ControlProperties {
            display: Patch::Set(None),
            text: Patch::Set(Some(text_with(vec![
                context_item("a", Some(component_ref("comp", "state"))),
                context_item("b", Some(component_ref("comp", "label"))),
            ]))),
            primary_action: Patch::Set(Some(component_action(Some(component_ref(
                "comp", "action",
            ))))),
            secondary_action: Patch::Set(Some(component_action(Some(component_ref(
                "comp", "action",
            ))))),
            ..Default::default()
        },
    );

    let usage = project.collect_component_usage();
    assert_eq!(usage.len(), 4, "two context items and two actions");
}

#[test]
fn a_reference_in_a_template_is_collected_too() {
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
        display_properties(Some(component_ref("comp", "state"))),
    );

    let usage = project.collect_component_usage();
    assert_eq!(usage.len(), 1);
    assert_eq!(
        usage[0].path.view_key().unwrap(),
        template_view("tpl"),
        "the path starts at the template it lives in"
    );
}

#[test]
fn a_reference_to_an_enclosing_export_is_not_a_component_usage() {
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

    assert!(
        project.collect_component_usage().is_empty(),
        "it points at the template, not at a component"
    );
    assert_eq!(
        display_member(&project, &template_view("tpl"), "ctl"),
        Some(MemberRef::OwnerExport(export_id("exp"))),
        "and it is still there"
    );
}

#[test]
fn an_unbound_reference_is_not_a_usage() {
    assert!(
        project_with_view_contents()
            .collect_component_usage()
            .is_empty()
    );
}

// ---------------------------------------------------------------------------
// Clearing usage
// ---------------------------------------------------------------------------

#[test]
fn two_usages_in_one_view_both_clear_and_emit_one_change() {
    // The second usage has to see the view the first one produced. Reading the
    // original instead loses one of the two clears.
    let project = project_with_two_usages();
    let usage = project.collect_component_usage();

    let mut changes = Vec::new();
    let cleared = project
        .clear_component_usage(&mut changes, &usage)
        .unwrap()
        .expect("something changed");

    let control = cleared
        .windows()
        .get_required(&window_id("win"))
        .unwrap()
        .controls
        .get_required(&control_id("ctl"))
        .unwrap()
        .clone();

    assert!(control.display.as_ref().unwrap().member.is_none());
    assert!(
        control
            .primary_action
            .as_ref()
            .unwrap()
            .component
            .as_ref()
            .unwrap()
            .member
            .is_none()
    );

    assert_eq!(
        kinds(&changes),
        ["set-window"],
        "one change for the view, not one per usage"
    );
}

#[test]
fn clearing_nothing_reports_nothing() {
    let mut changes = Vec::new();
    let result = project_with_two_usages()
        .clear_component_usage(&mut changes, &[])
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn clearing_usage_leaves_references_to_other_members_alone() {
    let project = project_with_two_usages();

    let display_only: Vec<_> = project
        .collect_component_usage()
        .into_iter()
        .filter(|item| item.member_name == "state")
        .collect();

    let cleared = project
        .clear_component_usage(&mut Vec::new(), &display_only)
        .unwrap()
        .expect("something changed");

    let control = cleared
        .windows()
        .get_required(&window_id("win"))
        .unwrap()
        .controls
        .get_required(&control_id("ctl"))
        .unwrap()
        .clone();

    assert!(control.display.as_ref().unwrap().member.is_none());
    assert!(
        control
            .primary_action
            .as_ref()
            .unwrap()
            .component
            .as_ref()
            .unwrap()
            .member
            .is_some(),
        "the action was not in the list"
    );
}

// ---------------------------------------------------------------------------
// The refresh chain
// ---------------------------------------------------------------------------

fn component_set(members: &[(&str, MemberType, &str)], version: &str) -> NewComponentData {
    let plugin = Plugin {
        name: String::from("name"),
        module: String::from("module"),
        version: String::from(version),
        description: None,
        members: members
            .iter()
            .map(|(name, member_type, value_type)| {
                (String::from(*name), member(*member_type, value_type))
            })
            .collect(),
        instance_name: String::from("instance"),
    };

    NewComponentData::new(
        Collection::from_iter([(
            component_id("comp"),
            Arc::new(Component {
                plugin: plugin_id(),
            }),
        )]),
        Collection::from_iter([(plugin_id(), Arc::new(plugin))]),
    )
}

/// The whole chain: work out what breaks, hand the payload back as the client
/// would, and apply it.
fn refresh(project: &Project, data: NewComponentData) -> (Vec<String>, Project) {
    let usage = project.collect_component_usage();
    let (breaking, server_data) = component::prepare_refresh(project, &usage, data).unwrap();

    let refresh = Refresh::parse(server_data).unwrap();
    let applied = refresh.apply(project, &mut Vec::new()).unwrap();

    (
        breaking
            .into_iter()
            .map(|operation| format!("{}:{}", operation.operation, operation.component_id))
            .collect(),
        applied,
    )
}

#[test]
fn an_unchanged_plugin_breaks_nothing() {
    let project = project_with_two_usages();

    let (breaking, applied) = refresh(
        &project,
        component_set(
            &[
                ("state", MemberType::State, "bool"),
                ("action", MemberType::Action, "bool"),
                ("level", MemberType::State, "range[0;100]"),
                ("label", MemberType::State, "text"),
            ],
            "1.0.0",
        ),
    );

    assert!(breaking.is_empty());
    assert_eq!(
        display_member(&applied, &window_view("win"), "ctl"),
        Some(component_ref("comp", "state")),
        "nothing was cleared"
    );
}

#[test]
fn a_component_that_is_gone_breaks_the_references_to_it() {
    let project = project_with_two_usages();

    let (breaking, applied) = refresh(
        &project,
        NewComponentData::new(Collection::new(), Collection::new()),
    );

    assert_eq!(breaking, ["remove:comp"]);
    assert!(applied.components().is_empty());
    assert_eq!(
        display_member(&applied, &window_view("win"), "ctl"),
        None,
        "the reference went with it"
    );
}

#[test]
fn a_member_a_reference_uses_disappearing_is_breaking() {
    let project = project_with_two_usages();

    let (breaking, applied) = refresh(
        &project,
        component_set(&[("action", MemberType::Action, "bool")], "2.0.0"),
    );

    assert_eq!(breaking, ["update:comp"]);
    assert_eq!(
        display_member(&applied, &window_view("win"), "ctl"),
        None,
        "the display pointed at 'state', which is gone"
    );
}

#[test]
fn a_member_widening_its_type_is_not_breaking() {
    // enum -> text and range -> wider range are promotions: whatever the old
    // member produced, the new one still accepts.
    let project = project_with_view_contents();
    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        display_properties(Some(component_ref("comp", "label"))),
    );

    let (breaking, applied) = refresh(
        &project,
        component_set(
            &[
                ("label", MemberType::State, "text"),
                ("state", MemberType::State, "bool"),
                ("action", MemberType::Action, "bool"),
                ("level", MemberType::State, "range[0;1000]"),
            ],
            "2.0.0",
        ),
    );

    assert!(breaking.is_empty());
    assert_eq!(
        display_member(&applied, &window_view("win"), "ctl"),
        Some(component_ref("comp", "label"))
    );
}

#[test]
fn a_member_narrowing_its_type_is_breaking() {
    let project = project_with_view_contents();
    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        display_properties(Some(component_ref("comp", "level"))),
    );

    let (breaking, _) = refresh(
        &project,
        component_set(&[("level", MemberType::State, "range[0;10]")], "2.0.0"),
    );

    assert_eq!(breaking, ["update:comp"]);
}

#[test]
fn a_member_nothing_references_can_change_freely() {
    let project = project_with_view_contents();
    let project = set_control(
        &project,
        &window_view("win"),
        &control_id("ctl"),
        display_properties(Some(component_ref("comp", "state"))),
    );

    let (breaking, _) = refresh(
        &project,
        component_set(&[("state", MemberType::State, "bool")], "2.0.0"),
    );

    assert!(
        breaking.is_empty(),
        "'action', 'level' and 'label' are gone, and nothing pointed at them"
    );
}

#[test]
fn applying_a_refresh_announces_the_new_component_set() {
    let project = project_with_two_usages();
    let usage = project.collect_component_usage();
    let (_, server_data) = component::prepare_refresh(
        &project,
        &usage,
        NewComponentData::new(Collection::new(), Collection::new()),
    )
    .unwrap();

    let mut changes = Vec::new();
    Refresh::parse(server_data)
        .unwrap()
        .apply(&project, &mut changes)
        .unwrap();

    assert_eq!(
        kinds(&changes),
        ["set-component-data", "set-window"],
        "the new data, then the view whose references it broke"
    );
}

#[test]
fn replacing_the_component_set_is_one_operation_with_its_change() {
    let (components, plugins) = component_data();

    let mut changes = Vec::new();
    let project = project().set_component_data(&mut changes, components, plugins);

    assert_eq!(project.components().len(), 1);
    assert_eq!(kinds(&changes), ["set-component-data"]);
}
