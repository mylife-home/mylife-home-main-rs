// Note : this also test runtime, but is easier to implement here than in core_plugin_runtime

use std::sync::Mutex;

use core_plugin_macros::{MylifePlugin, mylife_actions};
use core_plugin_runtime::{
    MylifePlugin, MylifePluginHooks, State,
    runtime::{Config, ConfigValue, MylifePluginRuntime, Value},
};

mod utils;

#[derive(Clone, Debug, PartialEq, Eq)]
enum HistoryItem {
    Init(String),
    Action(String),
}

struct History(Mutex<Vec<HistoryItem>>);

impl History {
    fn new() -> Self {
        History(Mutex::new(Vec::new()))
    }

    fn clear(&self) {
        (*self.0.lock().unwrap()).clear();
    }

    fn add(&self, item: HistoryItem) {
        (*self.0.lock().unwrap()).push(item)
    }

    fn get_all(&self) -> Vec<HistoryItem> {
        (*self.0.lock().unwrap()).clone()
    }
}

lazy_static::lazy_static! {
  static ref HISTORY: History = History::new();
}

#[derive(MylifePlugin, Default, Debug)]
#[mylife_plugin(usage = "logic")]
struct TestPlugin {
    #[mylife_config]
    config_value: String,

    #[mylife_state]
    state_value: State<bool>,
}

impl MylifePluginHooks for TestPlugin {
    fn new(_id: &str) -> Self {
        TestPlugin::default()
    }

    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        HISTORY.add(HistoryItem::Init(self.config_value.clone()));

        Ok(())
    }
}

#[mylife_actions]
impl TestPlugin {
    #[mylife_action]
    fn action_value(&mut self, arg: String) {
        HISTORY.add(HistoryItem::Action(arg));
    }

    #[mylife_action]
    fn set_state(&mut self, arg: bool) {
        self.state_value.set(arg)
    }
}

#[test]
fn test_behavior() {
    let runtime: Box<dyn MylifePluginRuntime> = TestPlugin::runtime();
    let mut component = runtime.create("comp-id");

    HISTORY.clear();

    let mut config = Config::new();
    config.insert(
        "configValue".to_string(),
        ConfigValue::String("config-value".into()),
    );

    component.configure(&config).unwrap();
    component.init().unwrap();
    component
        .execute_action("actionValue", Value::Text("action-arg".into()))
        .unwrap();

    assert_eq!(
        HISTORY.get_all(),
        vec![
            HistoryItem::Init("config-value".into()),
            HistoryItem::Action("action-arg".into()),
        ]
    );

    // on state
    assert_eq!(
        component.get_state("stateValue").unwrap(),
        Value::Bool(false)
    );
    component
        .execute_action("setState", Value::Bool(true))
        .unwrap();
    assert_eq!(
        component.get_state("stateValue").unwrap(),
        Value::Bool(true)
    );
}
