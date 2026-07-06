use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct FloatAverage {
    id: String,

    #[mylife_config]
    used_count: i64,

    state: Vec<f64>,

    #[mylife_state]
    value: State<f64>,
}

impl MylifePluginHooks for FloatAverage {
    type Error = Infallible;

    fn new(id: &str, _waker: WakeHandle) -> Self {
        Self {
            id: String::from(id),
            ..Default::default()
        }
    }

    fn init(&mut self) -> Result<(), Self::Error> {
        self.state = vec![f64::NAN; self.used_count.max(0) as usize];
        self.recompute();
        Ok(())
    }
}

#[mylife_actions]
impl FloatAverage {
    #[mylife_action]
    fn set_0(&mut self, arg: f64) {
        self.set(0, arg);
    }

    #[mylife_action]
    fn set_1(&mut self, arg: f64) {
        self.set(1, arg);
    }

    #[mylife_action]
    fn set_2(&mut self, arg: f64) {
        self.set(2, arg);
    }

    #[mylife_action]
    fn set_3(&mut self, arg: f64) {
        self.set(3, arg);
    }

    #[mylife_action]
    fn set_4(&mut self, arg: f64) {
        self.set(4, arg);
    }

    #[mylife_action]
    fn set_5(&mut self, arg: f64) {
        self.set(5, arg);
    }

    #[mylife_action]
    fn set_6(&mut self, arg: f64) {
        self.set(6, arg);
    }

    #[mylife_action]
    fn set_7(&mut self, arg: f64) {
        self.set(7, arg);
    }

    fn set(&mut self, index: usize, arg: f64) {
        if index >= self.state.len() {
            tracing::warn!(
                id = self.id,
                index,
                used_count = self.state.len(),
                "got set_<index> called, which is above used_count"
            );
            return;
        }

        self.state[index] = arg;
        self.recompute();
    }

    fn recompute(&mut self) {
        let value = if self.state.is_empty() {
            f64::NAN
        } else {
            self.state.iter().sum::<f64>() / self.state.len() as f64
        };

        self.value.set(value);
    }
}
