use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug)]
#[mylife_plugin(usage = "ui")]
pub struct UiStateText {
    #[mylife_state]
    value: State<String>,
}

impl MylifePluginHooks for UiStateText {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        UiStateText {
            value: Default::default(),
        }
    }
}

#[mylife_actions]
impl UiStateText {
    #[mylife_action]
    fn action(&mut self, arg: String) {
        self.value.set(arg);
    }
}
