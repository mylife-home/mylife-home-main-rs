use log::debug;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State};

const LOG_TARGET: &str = "mylife:home:core:plugins:logic-base:value-binary";

#[derive(MylifePlugin, Debug)]
#[mylife_plugin(description = "step relay", usage = "logic")] // name=
pub struct ValueBinary {
    id: String,

    #[mylife_config(description = "initial value (useless only config example")] // type=, name=
    config: bool,

    #[mylife_state(description = "actual value")] // type=, name=
    state: State<bool>,
}

// impl Drop si besoin de terminate
impl MylifePluginHooks for ValueBinary {
    fn new(id: &str) -> Self {
        ValueBinary {
            id: String::from(id),
            config: Default::default(),
            state: Default::default(),
        }
    }

    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.state.set(self.config);

        debug!(target: LOG_TARGET, "[{}] initial state = {}", self.id.as_str(), self.state.get());

        Ok(())
    }
}

#[mylife_actions]
impl ValueBinary {
    // can return Result<(), Box<dyn std::error::Error>> or nothing
    #[mylife_action(description = "set value to on")] // type=, name=
    fn on(&mut self, arg: bool) -> Result<(), Box<dyn std::error::Error>> {
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
