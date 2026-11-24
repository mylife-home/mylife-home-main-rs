//! The deploy-time id simulation: a template's components are instantiated
//! once per component using the template, and two chains can land on the same
//! final id.

use std::sync::Arc;

use super::fixture::*;

/// A second component using `tpl`, so the template is instantiated twice.
fn used_twice() -> Project {
    project_with_template()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("other"),
            template_definition("tpl"),
            Position::default(),
        )
        .unwrap()
}

#[test]
fn a_template_can_be_used_twice_when_its_components_carry_the_placeholder() {
    let project = used_twice();

    assert_eq!(
        component_ids(&project, &ViewKey::Project),
        ["other", "usage"]
    );
}

#[test]
fn a_fixed_id_inside_a_template_stops_it_being_used_twice() {
    // This is the consequence worth knowing: a component with a fixed id
    // inside a template generates exactly that id for every instance, so the
    // second instance is refused.
    let project = project()
        .set_template(&mut Vec::new(), template_id("tpl"))
        .unwrap()
        .set_component(
            &mut Vec::new(),
            &template_view("tpl"),
            component_id("fixed"),
            plugin_definition(),
            Position::default(),
        )
        .unwrap()
        .set_component(
            &mut Vec::new(),
            &ViewKey::Project,
            component_id("first"),
            template_definition("tpl"),
            Position::default(),
        )
        .unwrap();

    let mut changes = Vec::new();
    let error = project
        .set_component(
            &mut changes,
            &ViewKey::Project,
            component_id("second"),
            template_definition("tpl"),
            Position::default(),
        )
        .unwrap_err();

    let CoreModelError::DuplicateGeneratedIds { report } = error else {
        panic!("expected a duplicate report");
    };

    assert!(report.contains("id duplicate: fixed"), "{report}");
    assert!(changes.is_empty(), "nothing is written before the check");
}

#[test]
fn the_report_names_every_chain_that_produced_the_id() {
    let project = used_twice();

    let mut dry_run = NamingDryRun::new(&project);
    dry_run.set_component(
        &template_view("tpl"),
        component_id("fixed"),
        &plugin_definition(),
    );

    let CoreModelError::DuplicateGeneratedIds { report } = dry_run.validate().unwrap_err() else {
        panic!("expected a duplicate report");
    };

    assert!(report.contains("other -> fixed"), "{report}");
    assert!(report.contains("usage -> fixed"), "{report}");
}

#[test]
fn the_report_is_the_same_every_time_it_is_produced() {
    // Sorted, so a message a user is shown does not reorder itself between
    // two runs of the same check.
    let project = used_twice();

    let report = |()| {
        let mut dry_run = NamingDryRun::new(&project);
        dry_run.set_component(
            &template_view("tpl"),
            component_id("fixed"),
            &plugin_definition(),
        );
        match dry_run.validate().unwrap_err() {
            CoreModelError::DuplicateGeneratedIds { report } => report,
            other => panic!("expected a duplicate report, got {other}"),
        }
    };

    assert_eq!(report(()), report(()));
}

#[test]
fn an_id_is_folded_through_every_level_it_passes() {
    // `{{id}}-inner` inside a template used by `usage` deploys as
    // `usage-inner`, and a second level folds again.
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

    // Nothing collides, so the whole arrangement is accepted.
    assert!(NamingDryRun::new(&project).validate().is_ok());

    // Adding a component whose id is exactly what one chain folds to does
    // collide.
    let mut dry_run = NamingDryRun::new(&project);
    dry_run.set_component(
        &ViewKey::Project,
        component_id("top-mid-inner"),
        &plugin_definition(),
    );

    let CoreModelError::DuplicateGeneratedIds { report } = dry_run.validate().unwrap_err() else {
        panic!("expected a duplicate report");
    };

    assert!(report.contains("id duplicate: top-mid-inner"), "{report}");
}

#[test]
fn a_cycle_in_stored_data_is_reported_rather_than_followed() {
    // `set_component` refuses to build one, so this is only reachable from a
    // project file written by hand. The walk has to stop rather than recurse
    // until it runs out of stack.
    let instantiates_tpl = || {
        Arc::new(Component {
            definition: template_definition("tpl"),
            position: Position::default(),
            config: Some(Config::default()),
            external: false,
        })
    };

    let template = Template {
        components: Collection::from_iter([(component_id("loop"), instantiates_tpl())]),
        bindings: Collection::new(),
        exports: TemplateExports::default(),
    };

    let project = Project::from_parts(
        Collection::from_iter([(component_id("root"), instantiates_tpl())]),
        Collection::new(),
        Collection::new(),
        Collection::from_iter([(template_id("tpl"), Arc::new(template))]),
    );

    assert!(matches!(
        NamingDryRun::new(&project).validate(),
        Err(CoreModelError::CircularTemplate { .. })
    ));
}
