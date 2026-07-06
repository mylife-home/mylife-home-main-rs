use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct ByteToPercent {
    #[mylife_state(r#type = "range[0;100]")]
    value: State<i64>,
}

impl MylifePluginHooks for ByteToPercent {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }
}

#[mylife_actions]
impl ByteToPercent {
    #[mylife_action(r#type = "range[0;255]")]
    fn set(&mut self, arg: i64) {
        self.value.set(arg * 100 / 255);
    }
}
