use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct BinaryToPulse {
    #[mylife_state]
    off: State<bool>,

    #[mylife_state]
    on: State<bool>,
}

impl MylifePluginHooks for BinaryToPulse {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }
}

#[mylife_actions]
impl BinaryToPulse {
    #[mylife_action]
    fn action(&mut self, arg: bool) {
        if arg {
            self.on.set(true);
            self.on.set(false);
        } else {
            self.off.set(true);
            self.off.set(false);
        }
    }
}
