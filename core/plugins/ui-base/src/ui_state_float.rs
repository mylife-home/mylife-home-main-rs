use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug)]
#[mylife_plugin(usage = "ui")]
pub struct UiStateFloat {
    #[mylife_state]
    value: State<f64>,
}

impl MylifePluginHooks for UiStateFloat {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        UiStateFloat {
            value: Default::default(),
        }
    }
}

#[mylife_actions]
impl UiStateFloat {
    #[mylife_action]
    fn action(&mut self, arg: f64) {
        self.value.set(arg);
    }
}
