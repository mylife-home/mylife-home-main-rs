use plugin_macros::{mylife_actions, MylifePlugin};
use plugin_runtime::{
    metadata::{ConfigType, PluginUsage, Type},
    runtime::MylifePluginRuntime,
    MylifePlugin, MylifePluginHooks, State,
};

use crate::utils::TestMetadata;

mod utils;

// TODO: this one also act as example, but example should be put aside:
// - no default (not recommanded since it offers a public ctor with no args)
// - Drop impl
// - action with and without result

#[derive(MylifePlugin)]
#[mylife_plugin(
    name = "plugin-name",
    description = "plugin description",
    usage = "logic"
)]
struct TestPlugin {
    #[mylife_config(
        name = "configName",
        description = "config description",
        r#type = "bool"
    )]
    config_value: bool,

    #[mylife_state(name = "stateName", description = "state description", r#type = "bool")]
    state_value: State<bool>,
}

// impl Drop si besoin de terminate
impl MylifePluginHooks for TestPlugin {
    fn new(_id: &str) -> Self {
        TestPlugin {
            config_value: Default::default(),
            state_value: Default::default(),
        }
    }

    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

impl Drop for TestPlugin {
    fn drop(&mut self) {}
}

#[mylife_actions]
impl TestPlugin {
    // can return Result<(), Box<dyn std::error::Error>> or nothing
    #[mylife_action(
        name = "actionName",
        description = "action description",
        r#type = "bool"
    )]
    fn action_value(&mut self, _arg: bool) {}
}

#[test]
fn test_basic() {
    let runtime: Box<dyn MylifePluginRuntime> = TestPlugin::runtime();
    let meta = runtime.metadata();

    let mut expected = TestMetadata::new(
        "plugin-name",
        Some("plugin description"),
        PluginUsage::Logic,
    );

    expected.add_config("configName", Some("config description"), ConfigType::Bool);
    expected.add_state("stateName", Some("state description"), Type::Bool);
    expected.add_action("actionName", Some("action description"), Type::Bool);

    assert_eq!(TestMetadata::from_metadata(meta), expected);
}
