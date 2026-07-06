use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct BoolNot {
    #[mylife_state]
    value: State<bool>,
}

impl MylifePluginHooks for BoolNot {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }
}

#[mylife_actions]
impl BoolNot {
    #[mylife_action]
    fn set(&mut self, arg: bool) {
        self.value.set(!arg);
    }
}
