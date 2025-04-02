use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{
    MylifePlugin, MylifePluginHooks, State,
    metadata::{ConfigType, PluginUsage, Type},
    runtime::MylifePluginRuntime,
};

use crate::utils::TestMetadata;

mod utils;

#[derive(MylifePlugin, Default, Debug)]
#[mylife_plugin(name = "plugin-name", usage = "logic")]
struct TestPlugin {
    #[mylife_config(name = "configName", r#type = "bool")]
    config_value: bool,

    #[mylife_state(name = "stateName", r#type = "bool")]
    state_value: State<bool>,
}

impl MylifePluginHooks for TestPlugin {
    fn new(_id: &str) -> Self {
        TestPlugin::default()
    }

    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

#[mylife_actions]
impl TestPlugin {
    #[mylife_action(name = "actionName", r#type = "bool")]
    fn action_value(&mut self, _arg: bool) {}
}

#[test]
fn test_no_desc() {
    let runtime: Box<dyn MylifePluginRuntime> = TestPlugin::runtime();
    let meta = runtime.metadata();

    let mut expected = TestMetadata::new("plugin-name", None, PluginUsage::Logic);
    expected.add_config("configName", None, ConfigType::Bool);
    expected.add_state("stateName", None, Type::Bool);
    expected.add_action("actionName", None, Type::Bool);

    assert_eq!(TestMetadata::from_metadata(meta), expected);
}
