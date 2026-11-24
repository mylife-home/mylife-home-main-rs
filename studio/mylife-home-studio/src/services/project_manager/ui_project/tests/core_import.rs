//! Taking the component set from a core project.
//!
//! The core project is flattened first, so what the designer sees is what will
//! exist at runtime rather than what the core designer drew.

use std::collections::HashMap;
use std::sync::Arc;

use studio_web_api::component_model::PluginUsage;

use super::super::component::load_core_project_component_data;
use super::fixture::*;

use crate::services::project_manager::core_project::model as core;

fn core_plugin(name: &str, usage: PluginUsage) -> Arc<core::Plugin> {
    Arc::new(core::Plugin {
        name: String::from(name),
        module: String::from("module"),
        usage,
        version: String::from("1.0.0"),
        description: None,
        members: HashMap::from([(String::from("state"), member(MemberType::State, "bool"))]),
        config: HashMap::new(),
        instance_name: core::InstanceId::new("inst").unwrap(),
        toolbox_display: core::ToolboxDisplay::Show,
    })
}

fn core_component(definition: core::ComponentDefinition) -> Arc<core::Component> {
    Arc::new(core::Component {
        definition,
        position: core::Position::default(),
        config: Some(core::Config::default()),
        external: false,
    })
}

/// A core project with a UI plugin and a non-UI one, where the UI component
/// sits inside a template rather than on the project view.
fn core_project() -> core::Project {
    let ui_plugin = core::PluginId::new("inst:module.screen").unwrap();
    let logic_plugin = core::PluginId::new("inst:module.logic").unwrap();

    let template = core::Template {
        components: core::Collection::from_iter([(
            core::ComponentId::new("{{id}}-inner").unwrap(),
            core_component(core::ComponentDefinition::Plugin(ui_plugin.clone())),
        )]),
        bindings: core::Collection::new(),
        exports: core::TemplateExports::default(),
    };

    core::Project::from_parts(
        core::Collection::from_iter([
            (
                core::ComponentId::new("usage").unwrap(),
                core_component(core::ComponentDefinition::Template(
                    core::TemplateId::new("tpl").unwrap(),
                )),
            ),
            (
                core::ComponentId::new("brain").unwrap(),
                core_component(core::ComponentDefinition::Plugin(logic_plugin.clone())),
            ),
        ]),
        core::Collection::new(),
        core::Collection::from_iter([
            (ui_plugin, core_plugin("screen", PluginUsage::Ui)),
            (logic_plugin, core_plugin("logic", PluginUsage::Logic)),
        ]),
        core::Collection::from_iter([(core::TemplateId::new("tpl").unwrap(), Arc::new(template))]),
    )
}

#[test]
fn a_component_inside_a_template_arrives_under_the_id_it_will_have() {
    // `{{id}}-inner` inside `tpl`, used by `usage`, runs as `usage-inner`.
    // Binding a control to `{{id}}-inner` would bind to nothing.
    let data = load_core_project_component_data(&core_project()).unwrap();

    assert_eq!(
        data.components
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["usage-inner"]
    );
}

#[test]
fn only_plugins_a_ui_can_talk_to_come_across() {
    // `brain` is a logic component: real at runtime, but nothing a control
    // can be bound to.
    let data = load_core_project_component_data(&core_project()).unwrap();

    assert_eq!(data.plugins.len(), 1);

    let plugin = data
        .plugins
        .get_required(&PluginId::new("inst:module.screen").unwrap())
        .unwrap();

    assert_eq!(plugin.name, "screen");
    assert_eq!(plugin.instance_name, "inst");
    assert!(plugin.members.contains_key("state"));
}

#[test]
fn each_component_points_at_the_plugin_it_instantiates() {
    let data = load_core_project_component_data(&core_project()).unwrap();

    let component = data
        .components
        .get_required(&ComponentId::new("usage-inner").unwrap())
        .unwrap();

    assert_eq!(
        component.plugin,
        PluginId::new("inst:module.screen").unwrap()
    );
}

#[test]
fn a_core_project_with_nothing_a_ui_can_use_comes_back_empty() {
    let logic_plugin = core::PluginId::new("inst:module.logic").unwrap();

    let project = core::Project::from_parts(
        core::Collection::from_iter([(
            core::ComponentId::new("brain").unwrap(),
            core_component(core::ComponentDefinition::Plugin(logic_plugin.clone())),
        )]),
        core::Collection::new(),
        core::Collection::from_iter([(logic_plugin, core_plugin("logic", PluginUsage::Logic))]),
        core::Collection::new(),
    );

    let data = load_core_project_component_data(&project).unwrap();

    assert!(data.components.is_empty());
    assert!(data.plugins.is_empty());
}
