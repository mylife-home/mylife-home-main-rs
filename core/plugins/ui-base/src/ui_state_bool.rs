use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "ui")]
pub struct UiStateBool {
    #[mylife_state]
    value: State<bool>,
}

impl MylifePluginHooks for UiStateBool {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }
}

#[mylife_actions]
impl UiStateBool {
    #[mylife_action]
    fn action(&mut self, arg: bool) {
        self.value.set(arg);
    }
}
