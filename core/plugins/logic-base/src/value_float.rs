use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct ValueFloat {
    #[mylife_state]
    value: State<f64>,
}

impl MylifePluginHooks for ValueFloat {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }

    fn init(&mut self) -> Result<(), Self::Error> {
        self.value.set(f64::NAN);
        Ok(())
    }
}

#[mylife_actions]
impl ValueFloat {
    #[mylife_action]
    fn set_value(&mut self, arg: f64) {
        self.value.set(arg);
    }
}
