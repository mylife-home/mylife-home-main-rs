//! Plugins, the instances they are derived from, and the toolbox operations.

use super::fixture::*;

fn displayed(project: &Project, id: &PluginId) -> ToolboxDisplay {
    project.plugins().get_required(id).unwrap().toolbox_display
}

fn second_plugin() -> PluginId {
    PluginId::build(&instance_id("inst"), "module", "second").unwrap()
}

fn other_instance_plugin() -> PluginId {
    PluginId::build(&instance_id("other"), "module", "name").unwrap()
}

#[test]
fn instances_are_derived_from_the_plugins() {
    // Nothing stores an instance list, so there is no second structure that
    // can fall out of step with the plugins.
    let instances = project_with_two_instances().instances();

    assert_eq!(
        instances
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["inst", "other"],
        "in id order"
    );
    assert_eq!(instances[&instance_id("inst")].len(), 2);
    assert_eq!(instances[&instance_id("other")].len(), 1);
}

#[test]
fn an_instance_nothing_runs_does_not_exist() {
    assert!(matches!(
        project().instance_plugins(&instance_id("nope")),
        Err(CoreModelError::NotFound {
            kind: IdKind::Instance,
            ..
        })
    ));
}

#[test]
fn hiding_a_plugin_records_it_and_says_so() {
    let mut changes = Vec::new();
    let project = project()
        .set_plugin_display(&mut changes, &plugin_id(), ToolboxDisplay::Hide)
        .unwrap()
        .expect("the display changed");

    assert_eq!(displayed(&project, &plugin_id()), ToolboxDisplay::Hide);
    assert_eq!(kinds(&changes), ["set-plugin-display"]);
}

#[test]
fn hiding_a_plugin_that_is_already_hidden_changes_nothing() {
    let project = project()
        .set_plugin_display(&mut Vec::new(), &plugin_id(), ToolboxDisplay::Hide)
        .unwrap()
        .expect("the display changed");

    let mut changes = Vec::new();
    let result = project
        .set_plugin_display(&mut changes, &plugin_id(), ToolboxDisplay::Hide)
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn setting_the_display_of_an_unknown_plugin_is_refused() {
    assert!(matches!(
        project().set_plugin_display(
            &mut Vec::new(),
            &PluginId::new("nope").unwrap(),
            ToolboxDisplay::Hide
        ),
        Err(CoreModelError::NotFound {
            kind: IdKind::Plugin,
            ..
        })
    ));
}

#[test]
fn hiding_an_instance_hides_each_of_its_plugins_once() {
    let mut changes = Vec::new();
    let project = project_with_two_instances()
        .set_instance_display(&mut changes, &instance_id("inst"), ToolboxDisplay::Hide)
        .unwrap()
        .expect("something changed");

    assert_eq!(displayed(&project, &plugin_id()), ToolboxDisplay::Hide);
    assert_eq!(displayed(&project, &second_plugin()), ToolboxDisplay::Hide);
    assert_eq!(
        displayed(&project, &other_instance_plugin()),
        ToolboxDisplay::Show,
        "another instance is left alone"
    );

    assert_eq!(
        kinds(&changes),
        ["set-plugin-display", "set-plugin-display"],
        "one per plugin, and no more"
    );
}

#[test]
fn hiding_an_instance_already_hidden_changes_nothing() {
    let project = project_with_two_instances()
        .set_instance_display(&mut Vec::new(), &instance_id("inst"), ToolboxDisplay::Hide)
        .unwrap()
        .expect("something changed");

    let mut changes = Vec::new();
    let result = project
        .set_instance_display(&mut changes, &instance_id("inst"), ToolboxDisplay::Hide)
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}

#[test]
fn deleting_an_unused_plugin_removes_it_and_says_so() {
    let mut changes = Vec::new();
    let project = project()
        .clear_plugin(&mut changes, &plugin_id())
        .unwrap()
        .expect("it was there");

    assert!(project.plugins().is_empty());
    assert_eq!(kinds(&changes), ["clear-plugin"]);
}

#[test]
fn a_plugin_a_component_instantiates_cannot_be_deleted() {
    let mut changes = Vec::new();
    let error = project_with_components()
        .clear_plugin(&mut changes, &plugin_id())
        .unwrap_err();

    assert!(matches!(
        error,
        CoreModelError::InUse {
            kind: IdKind::Plugin,
            ..
        }
    ));
    assert!(changes.is_empty(), "a refused operation emits nothing");
}

#[test]
fn deleting_an_unknown_plugin_is_refused() {
    assert!(matches!(
        project().clear_plugin(&mut Vec::new(), &PluginId::new("nope").unwrap()),
        Err(CoreModelError::NotFound {
            kind: IdKind::Plugin,
            ..
        })
    ));
}

#[test]
fn clearing_an_instance_drops_only_the_plugins_nothing_uses() {
    // This is what makes it different from deleting one plugin by hand: a
    // plugin still in use is skipped rather than refusing the whole thing.
    let project = project_with_two_instances()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("comp"),
            plugin_definition(),
            Position::default(),
        )
        .unwrap();

    let mut changes = Vec::new();
    let project = project
        .clear_unused_instance_plugins(&mut changes, &instance_id("inst"))
        .unwrap()
        .expect("something changed");

    assert!(
        project.plugins().contains(&plugin_id()),
        "the one in use stays"
    );
    assert!(
        !project.plugins().contains(&second_plugin()),
        "the unused one goes"
    );
    assert_eq!(kinds(&changes), ["clear-plugin"]);
}

#[test]
fn clearing_an_instance_whose_plugins_are_all_in_use_changes_nothing() {
    let project = project()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("comp"),
            plugin_definition(),
            Position::default(),
        )
        .unwrap();

    let mut changes = Vec::new();
    let result = project
        .clear_unused_instance_plugins(&mut changes, &instance_id("inst"))
        .unwrap();

    assert!(result.is_none());
    assert!(changes.is_empty());
}
