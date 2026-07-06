use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "ui")]
pub struct UiStateNullablePercent {
    #[mylife_state(r#type = "range[-1;100]")]
    value: State<i64>,
}

impl MylifePluginHooks for UiStateNullablePercent {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }
}

#[mylife_actions]
impl UiStateNullablePercent {
    #[mylife_action(r#type = "range[-1;100]")]
    fn action(&mut self, arg: i64) {
        self.value.set(arg);
    }
}
