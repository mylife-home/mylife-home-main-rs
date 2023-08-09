use plugin_macros::{mylife_actions, MylifePlugin};
use plugin_runtime::{
    metadata::{ConfigType, PluginUsage, Type},
    runtime::MylifePluginRuntime,
    MylifePlugin, MylifePluginHooks, State,
};

use crate::utils::TestMetadata;

mod utils;

#[derive(MylifePlugin)]
#[mylife_plugin(
    name = "plugin-name",
    description = "plugin description",
    usage = "logic"
)]
pub struct Basic {
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
impl MylifePluginHooks for Basic {
    fn new(_id: &str) -> Self {
        Basic {
            config_value: Default::default(),
            state_value: Default::default(),
        }
    }

    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

impl Drop for Basic {
    fn drop(&mut self) {}
}

#[mylife_actions]
impl Basic {
    // can return Result<(), Box<dyn std::error::Error>> or nothing
    #[mylife_action(
        name = "actionName",
        description = "action description",
        r#type = "bool"
    )]
    fn action_value(&mut self, _arg: bool) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

#[test]
fn test_basic() {
    let runtime: Box<dyn MylifePluginRuntime> = Basic::runtime();
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

// TODO
// test auto naming
// test auto type
// pas de description
