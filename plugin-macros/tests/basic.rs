use plugin_macros::{mylife_actions, MylifePlugin};
use plugin_runtime::{MylifePluginHooks, State, MylifePlugin, runtime::MylifePluginRuntime, metadata::{PluginUsage, ConfigType, MemberType, Type}};

#[derive(MylifePlugin)]
#[mylife_plugin(name = "plugin-name", description = "plugin description", usage = "logic")]
pub struct Basic {
    #[mylife_config(name = "configName", description = "config description", r#type="bool")]
    config_value: bool,

    #[mylife_state(name = "stateName", description = "state description", r#type="bool")]
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
  fn drop(&mut self) {
  }
}

#[mylife_actions]
impl Basic {
    // can return Result<(), Box<dyn std::error::Error>> or nothing
    #[mylife_action(name = "actionName", description = "action description", r#type="bool")]
    fn action_value(&mut self, _arg: bool) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

#[test]
fn test_basic() {
  let runtime: Box<dyn MylifePluginRuntime> = Basic::runtime();
  let meta = runtime.metadata();

  assert_eq!(meta.name(), "plugin-name");
  assert_eq!(meta.description().unwrap_or("<no desc>"), "plugin description");
  assert_eq!(meta.usage(), PluginUsage::Logic);

  assert_eq!(meta.config().len(), 1);
  let (name, config_item) = meta.config().iter().next().unwrap();
  assert_eq!(name, "configName");
  assert_eq!(config_item.description().unwrap_or("<no desc>"), "config description");
  assert_eq!(config_item.value_type(), ConfigType::Bool);

  assert_eq!(meta.members().len(), 2);
  let mut keys = Vec::from_iter(meta.members().keys());
  keys.sort();

  let name = keys[0];
  let member = meta.members().get(name).unwrap();
  assert_eq!(name, "actionName");
  assert_eq!(member.description().unwrap_or("<no desc>"), "action description");
  assert_eq!(member.member_type(), MemberType::Action);
  assert_eq!(*member.value_type(), Type::Bool);

  let name = keys[1];
  let member = meta.members().get(name).unwrap();
  assert_eq!(name, "stateName");
  assert_eq!(member.description().unwrap_or("<no desc>"), "state description");
  assert_eq!(member.member_type(), MemberType::State);
  assert_eq!(*member.value_type(), Type::Bool);

}

// TODO
// test auto naming
// test auto type
// pas de description