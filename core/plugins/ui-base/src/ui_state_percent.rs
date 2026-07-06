use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug)]
#[mylife_plugin(usage = "ui")]
pub struct UiStatePercent {
    #[mylife_state(r#type = "range[0;100]")]
    value: State<i64>,
}

impl MylifePluginHooks for UiStatePercent {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        UiStatePercent {
            value: Default::default(),
        }
    }
}

#[mylife_actions]
impl UiStatePercent {
    #[mylife_action(r#type = "range[0;100]")]
    fn action(&mut self, arg: i64) {
        self.value.set(arg);
    }
}
