use core_plugin_macros::{mylife_actions, MylifePlugin};
use core_plugin_runtime::{
    metadata::{ConfigType, PluginUsage, Type},
    runtime::MylifePluginRuntime,
    MylifePlugin, MylifePluginHooks, State,
};

use crate::utils::TestMetadata;

mod utils;

#[derive(MylifePlugin)]
#[mylife_plugin(
    name = "example-plugin", // Optional, infered from struct name
    description = "plugin description", // Optional
    usage = "logic"
)]
struct ExamplePlugin {
    #[allow(dead_code)]
    id: String, // May be kept for logging or whatever

    #[mylife_config(
        name = "configValue", // Optional, infered from field name
        description = "config description", // Optional
        r#type = "bool"
    )]
    config_value: bool,

    #[mylife_state(
      name = "stateValue", // Optional, infered from field name
      description = "state description", // Optional
      r#type = "bool"
    )]
    state_value: State<bool>,
}

impl MylifePluginHooks for ExamplePlugin {
    fn new(id: &str) -> Self {
        ExamplePlugin {
            id: String::from(id),
            config_value: Default::default(),
            state_value: Default::default(),
        }
    }

    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // This is executed after configuration has been set
        Ok(())
    }
}

// Drop implementation if cleanup tasks needed
impl Drop for ExamplePlugin {
    fn drop(&mut self) {
        // Cleanup tasks
    }
}

#[mylife_actions]
impl ExamplePlugin {
    // can return nothing
    #[mylife_action(
        name = "action1", // Optional, infered from function name
        description = "action description", // Optional
        r#type = "bool"
    )]
    fn action1(&mut self, _arg: bool) {
        // Do something
    }
}

// Can be another impl or same impl
#[mylife_actions]
impl ExamplePlugin {
    // can return Result<(), Box<dyn std::error::Error>>
    #[mylife_action(
        name = "action2", // Optional, infered from function name
        description = "action description", // Optional
        r#type = "bool"
    )]
    fn action2(&mut self, _arg: bool) -> Result<(), Box<dyn std::error::Error>> {
        // Do something
        Ok(())
    }
}

#[test]
fn test_example() {
    let runtime: Box<dyn MylifePluginRuntime> = ExamplePlugin::runtime();
    let meta = runtime.metadata();

    let mut expected = TestMetadata::new(
        "example-plugin",
        Some("plugin description"),
        PluginUsage::Logic,
    );

    expected.add_config("configValue", Some("config description"), ConfigType::Bool);
    expected.add_state("stateValue", Some("state description"), Type::Bool);
    expected.add_action("action1", Some("action description"), Type::Bool);
    expected.add_action("action2", Some("action description"), Type::Bool);

    assert_eq!(TestMetadata::from_metadata(meta), expected);
}
