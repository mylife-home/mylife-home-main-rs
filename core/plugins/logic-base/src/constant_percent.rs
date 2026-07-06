use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct ConstantPercent {
    #[mylife_config]
    config_value: i64,

    #[mylife_state(r#type = "range[0;100]")]
    value: State<i64>,
}

impl MylifePluginHooks for ConstantPercent {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }

    fn init(&mut self) -> Result<(), Self::Error> {
        let value = self.config_value.max(0).min(100);
        self.value.set(value);
        Ok(())
    }
}

#[mylife_actions]
impl ConstantPercent {}
