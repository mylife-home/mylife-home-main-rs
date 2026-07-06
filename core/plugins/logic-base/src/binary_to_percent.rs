use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct BinaryToPercent {
    #[mylife_config]
    low: i64,

    #[mylife_config]
    high: i64,

    #[mylife_state(r#type = "range[0;100]")]
    value: State<i64>,
}

impl MylifePluginHooks for BinaryToPercent {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }

    fn init(&mut self) -> Result<(), Self::Error> {
        self.low = self.low.max(0).min(100);
        self.high = self.high.max(0).min(100);
        self.value.set(self.low);
        Ok(())
    }
}

#[mylife_actions]
impl BinaryToPercent {
    #[mylife_action]
    fn set_value(&mut self, arg: bool) {
        self.value.set(if arg { self.high } else { self.low });
    }
}
