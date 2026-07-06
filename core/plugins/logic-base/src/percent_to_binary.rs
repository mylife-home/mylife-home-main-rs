use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct PercentToBinary {
    #[mylife_config]
    threshold: i64,

    #[mylife_state]
    value: State<bool>,
}

impl MylifePluginHooks for PercentToBinary {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }
}

#[mylife_actions]
impl PercentToBinary {
    #[mylife_action(r#type = "range[0;100]")]
    fn set_value(&mut self, arg: i64) {
        self.value.set(arg >= self.threshold);
    }
}
