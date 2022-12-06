
impl MylifeComponent {
  pub fn new(on_fail: fn(error: Box<dyn std::error::Error>), on_state: fn(state)) -> Self {
      MylifeComponent { on_fail, on_state }
  }

}

impl MylifePlugin {
  fn runtime() -> Box<dyn MyLifePluginRuntime>
}

// Trait implemented by the plugin itself
pub trait MylifePlugin: Default + MylifePluginHooks {
  // used to export
  fn runtime() -> Box<dyn MyLifePluginRuntime>;

  // mark the plugin instance like it has failed
  fn fail(error: Box<dyn std::error::Error>);
}

pub struct RuntimeBuilder {
  set_desc
  set_name
  add_state
  add_config
}