use core_plugin_macros::MylifePlugin;
use core_plugin_runtime::{
    metadata::{ConfigType, PluginUsage, Type},
    runtime::MylifePluginRuntime,
    MylifePlugin, MylifePluginHooks, State,
};

use crate::utils::TestMetadata;

mod utils;

#[derive(MylifePlugin, Default, Debug)]
#[mylife_plugin(
    name = "plugin-name",
    description = "plugin description",
    usage = "logic"
)]
struct TestPlugin {
    #[mylife_config(name = "configString", description = "config description")]
    config_string: String,

    #[mylife_config(name = "configBool", description = "config description")]
    config_bool: bool,

    #[mylife_config(name = "configInteger", description = "config description")]
    config_integer: i64,

    #[mylife_config(name = "configFloat", description = "config description")]
    config_float: f64,

    // Range: cannot infer
    #[mylife_state(name = "stateText", description = "state description")]
    state_text: State<String>,

    #[mylife_state(name = "stateFloat", description = "state description")]
    state_float: State<f64>,

    #[mylife_state(name = "stateBool", description = "state description")]
    state_bool: State<bool>,
    // Enum: cannot infer

    // Complex: not implemented
}

impl MylifePluginHooks for TestPlugin {
    fn new(_id: &str) -> Self {
        TestPlugin::default()
    }

    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

#[test]
fn test_typing() {
    let runtime: Box<dyn MylifePluginRuntime> = TestPlugin::runtime();
    let meta = runtime.metadata();

    let mut expected = TestMetadata::new(
        "plugin-name",
        Some("plugin description"),
        PluginUsage::Logic,
    );

    expected.add_config(
        "configString",
        Some("config description"),
        ConfigType::String,
    );
    expected.add_config("configBool", Some("config description"), ConfigType::Bool);
    expected.add_config(
        "configInteger",
        Some("config description"),
        ConfigType::Integer,
    );
    expected.add_config("configFloat", Some("config description"), ConfigType::Float);
    expected.add_state("stateText", Some("state description"), Type::Text);
    expected.add_state("stateFloat", Some("state description"), Type::Float);
    expected.add_state("stateBool", Some("state description"), Type::Bool);

    assert_eq!(TestMetadata::from_metadata(meta), expected);
}
