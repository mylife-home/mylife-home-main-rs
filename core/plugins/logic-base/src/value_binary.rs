use log::debug;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

const LOG_TARGET: &str = "mylife:home:core:plugins:logic-base:value-binary";

#[derive(MylifePlugin, Debug)]
#[mylife_plugin(description = "step relay", usage = "logic")] // name=
pub struct ValueBinary {
    id: String,

    #[mylife_config(description = "initial value (useless only config example")] // type=, name=
    config: bool,

    #[mylife_state(description = "actual value")] // type=, name=
    state: State<bool>,

    _waker: WakeHandle,
}

// impl Drop if terminate needed
impl MylifePluginHooks for ValueBinary {
    fn new(id: &str, waker: WakeHandle) -> Self {
        ValueBinary {
            id: String::from(id),
            config: Default::default(),
            state: Default::default(),
            _waker: waker,
        }
    }

    fn init(&mut self) -> anyhow::Result<()> {
        self.state.set(self.config);

        debug!(target: LOG_TARGET, "[{}] initial state = {}", self.id.as_str(), self.state.get());

        Ok(())
    }
}

#[mylife_actions]
impl ValueBinary {
    // can return anyhow::Result<()> or nothing
    #[mylife_action(description = "set value to on")] // type=, name=
    fn on(&mut self, arg: bool) -> anyhow::Result<()> {
        if arg {
            self.state.set(true);
        }

        Ok(())
    }

    #[mylife_action(description = "set value to off")]
    fn off(&mut self, arg: bool) {
        if arg {
            self.state.set(false);
        }
    }

    #[mylife_action(description = "toggle value")]
    fn toggle(&mut self, arg: bool) {
        if arg {
            self.state.set(!self.state.get());
        }
    }
}
