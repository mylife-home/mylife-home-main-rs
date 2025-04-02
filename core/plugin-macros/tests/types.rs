use plugin_macros::MylifePlugin;
use plugin_runtime::{
    MylifePlugin, MylifePluginHooks, State,
    metadata::{PluginUsage, Type},
    runtime::MylifePluginRuntime,
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
    #[mylife_state(
        name = "stateRange",
        description = "state description",
        r#type = "range[0;42]"
    )]
    state_range: State<i64>,

    #[mylife_state(name = "stateText", description = "state description", r#type = "text")]
    state_text: State<String>,

    #[mylife_state(
        name = "stateFloat",
        description = "state description",
        r#type = "float"
    )]
    state_float: State<f64>,

    #[mylife_state(name = "stateBool", description = "state description", r#type = "bool")]
    state_bool: State<bool>,

    #[mylife_state(
        name = "stateEnum",
        description = "state description",
        r#type = "enum{one,two,three}"
    )]
    state_enum: State<String>,
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

    expected.add_state("stateRange", Some("state description"), Type::Range(0, 42));
    expected.add_state("stateText", Some("state description"), Type::Text);
    expected.add_state("stateFloat", Some("state description"), Type::Float);
    expected.add_state("stateBool", Some("state description"), Type::Bool);
    expected.add_state(
        "stateEnum",
        Some("state description"),
        Type::Enum(vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
        ]),
    );

    assert_eq!(TestMetadata::from_metadata(meta), expected);
}
