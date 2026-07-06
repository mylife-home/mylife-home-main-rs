use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct PercentToByte {
    #[mylife_state(r#type = "range[0;255]")]
    value: State<i64>,
}

impl MylifePluginHooks for PercentToByte {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }
}

#[mylife_actions]
impl PercentToByte {
    #[mylife_action(r#type = "range[0;100]")]
    fn set_value(&mut self, arg: i64) {
        self.value.set(arg * 255 / 100);
    }
}
