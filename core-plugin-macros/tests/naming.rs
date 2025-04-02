use core_plugin_macros::{MylifePlugin, mylife_actions};
use core_plugin_runtime::{
    MylifePlugin, MylifePluginHooks, State,
    metadata::{ConfigType, PluginUsage, Type},
    runtime::MylifePluginRuntime,
};

use crate::utils::TestMetadata;

mod utils;

#[derive(MylifePlugin, Default, Debug)]
#[mylife_plugin(description = "plugin description", usage = "logic")]
struct PluginName {
    #[mylife_config(description = "config description", r#type = "bool")]
    config_name: bool,

    #[mylife_state(description = "state description", r#type = "bool")]
    state_name: State<bool>,
}

// impl Drop si besoin de terminate
impl MylifePluginHooks for PluginName {
    fn new(_id: &str) -> Self {
        PluginName::default()
    }

    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

#[mylife_actions]
impl PluginName {
    #[mylife_action(description = "action description", r#type = "bool")]
    fn action_name(&mut self, _arg: bool) {}
}

#[test]
fn test_naming() {
    let runtime: Box<dyn MylifePluginRuntime> = PluginName::runtime();
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
