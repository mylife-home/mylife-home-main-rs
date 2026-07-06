use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct SwitchToButton {
    switch_: bool,

    #[mylife_state]
    value: State<bool>,
}

impl MylifePluginHooks for SwitchToButton {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }
}

#[mylife_actions]
impl SwitchToButton {
    #[mylife_action]
    fn action(&mut self, arg: bool) {
        if self.switch_ == arg {
            return;
        }

        self.switch_ = arg;
        self.value.set(true);
        self.value.set(false);
    }
}
