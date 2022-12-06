use plugin_macros::{mylife_action, MylifePlugin};
use plugin_runtime::State;

#[derive(MylifePlugin)]
#[mylife_plugin(description = "step relay")] // name=
pub struct ValueBinary {
    #[mylife_config(description = "useless")] // type=, name=
    config: bool,

    #[mylife_state(description = "actual value")] // type=, name=
    state: State<bool>,
}

impl ValueBinary {
    #[mylife_action(description = "set value to on")] // type=, name=
    fn on(&mut self, arg: bool) {
        if arg {
            self.state.set(true);
        }
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
