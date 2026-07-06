use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct BoolOr {
    id: String,

    #[mylife_config]
    used_count: i64,

    state: Vec<bool>,

    #[mylife_state]
    value: State<bool>,
}

impl MylifePluginHooks for BoolOr {
    type Error = Infallible;

    fn new(id: &str, _waker: WakeHandle) -> Self {
        Self {
            id: String::from(id),
            ..Default::default()
        }
    }

    fn init(&mut self) -> Result<(), Self::Error> {
        self.state = vec![false; self.used_count.max(0) as usize];
        self.recompute();
        Ok(())
    }
}

#[mylife_actions]
impl BoolOr {
    #[mylife_action]
    fn set_0(&mut self, arg: bool) {
        self.set(0, arg);
    }

    #[mylife_action]
    fn set_1(&mut self, arg: bool) {
        self.set(1, arg);
    }

    #[mylife_action]
    fn set_2(&mut self, arg: bool) {
        self.set(2, arg);
    }

    #[mylife_action]
    fn set_3(&mut self, arg: bool) {
        self.set(3, arg);
    }

    #[mylife_action]
    fn set_4(&mut self, arg: bool) {
        self.set(4, arg);
    }

    #[mylife_action]
    fn set_5(&mut self, arg: bool) {
        self.set(5, arg);
    }

    #[mylife_action]
    fn set_6(&mut self, arg: bool) {
        self.set(6, arg);
    }

    #[mylife_action]
    fn set_7(&mut self, arg: bool) {
        self.set(7, arg);
    }

    fn set(&mut self, index: usize, arg: bool) {
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
        let value = self.state.iter().any(|item| *item);
        self.value.set(value);
    }
}
