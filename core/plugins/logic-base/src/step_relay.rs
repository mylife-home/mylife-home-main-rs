use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct StepRelay {
    #[mylife_state]
    value: State<bool>,
}

impl MylifePluginHooks for StepRelay {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }
}

#[mylife_actions]
impl StepRelay {
    #[mylife_action]
    fn action(&mut self, arg: bool) {
        if arg {
            self.value.set(!self.value.get());
        }
    }

    #[mylife_action]
    fn on(&mut self, arg: bool) {
        if arg {
            self.value.set(true);
        }
    }

    #[mylife_action]
    fn off(&mut self, arg: bool) {
        if arg {
            self.value.set(false);
        }
    }
}
