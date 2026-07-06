use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct ValueBinary {
    #[mylife_state]
    value: State<bool>,
}

impl MylifePluginHooks for ValueBinary {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }
}

#[mylife_actions]
impl ValueBinary {
    #[mylife_action]
    fn set_value(&mut self, arg: bool) {
        self.value.set(arg);
    }
}
