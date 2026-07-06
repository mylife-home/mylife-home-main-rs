use std::{convert::Infallible, f64};

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct FloatToNullablePercent {
    #[mylife_config]
    min: f64,

    #[mylife_config]
    max: f64,

    #[mylife_state(r#type = "range[-1;100]")]
    value: State<i64>,
}

impl MylifePluginHooks for FloatToNullablePercent {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }

    fn init(&mut self) -> Result<(), Self::Error> {
        self.value.set(-1);
        Ok(())
    }
}

#[mylife_actions]
impl FloatToNullablePercent {
    #[mylife_action]
    fn set_value(&mut self, arg: f64) {
        if arg.is_nan() {
            self.value.set(-1);
            return;
        }

        let reverse = self.min > self.max;
        let percent = if reverse {
            100 - self.compute(self.max, self.min, arg)
        } else {
            self.compute(self.min, self.max, arg)
        };
        self.value.set(percent);
    }

    fn compute(&self, min: f64, max: f64, arg: f64) -> i64 {
        let arg = arg.clamp(min, max);
        if (max - min).abs() < f64::EPSILON {
            return 0;
        }
        ((arg - min) * 100.0 / (max - min)).round() as i64
    }
}
