use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct ConstantBool {
    #[mylife_config]
    config_value: bool,

    #[mylife_state]
    value: State<bool>,
}

impl MylifePluginHooks for ConstantBool {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }

    fn init(&mut self) -> Result<(), Self::Error> {
        self.value.set(self.config_value);
        Ok(())
    }
}

#[mylife_actions]
impl ConstantBool {}
