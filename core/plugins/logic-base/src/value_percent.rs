use std::convert::Infallible;

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug, Default)]
#[mylife_plugin(usage = "logic")]
pub struct ValuePercent {
    #[mylife_config(
        name = "toggleThreshold",
        description = "Valeur partir de laquelle toggle passe à OFF ou ON. Typiquement 1 (Note: peut être écrasé par l'action 'setToggleThreshold'"
    )]
    config_toggle_threshold: i64,

    #[mylife_config(
        name = "onValue",
        description = "Valeur définie lorsqu'on passe à ON. Typiquement 100 (Note: peut être écrasé par l'action 'setOnValue'"
    )]
    config_on_value: i64,

    #[mylife_config(
        name = "offValue",
        description = "Valeur définie lorsqu'on passe à OFF. Typiquement 0 (Note: peut être écrasé par l'action 'setOffValue'"
    )]
    config_off_value: i64,

    #[mylife_state(
        r#type = "range[0;100]",
        description = "Valeur partir de laquelle toggle passe à OFF ou ON. Typiquement 1"
    )]
    toggle_threshold: State<i64>,

    #[mylife_state(
        r#type = "range[0;100]",
        description = "Valeur définie lorsqu'on passe à ON. Typiquement 100"
    )]
    on_value: State<i64>,

    #[mylife_state(
        r#type = "range[0;100]",
        description = "Valeur définie lorsqu'on passe à OFF. Typiquement 0"
    )]
    off_value: State<i64>,

    #[mylife_state(r#type = "range[0;100]")]
    value: State<i64>,
}

impl MylifePluginHooks for ValuePercent {
    type Error = Infallible;

    fn new(_id: &str, _waker: WakeHandle) -> Self {
        Default::default()
    }

    fn init(&mut self) -> Result<(), Self::Error> {
        self.toggle_threshold.set(self.config_toggle_threshold);
        self.on_value.set(self.config_on_value);
        self.off_value.set(self.config_off_value);
        self.value.set(*self.off_value.get());
        Ok(())
    }
}

#[mylife_actions]
impl ValuePercent {
    #[mylife_action(r#type = "range[0;100]")]
    fn set_value(&mut self, arg: i64) {
        self.value.set(arg);
    }

    #[mylife_action(r#type = "range[-1;100]")]
    fn set_pulse(&mut self, arg: i64) {
        if arg != -1 {
            self.value.set(arg);
        }
    }

    #[mylife_action]
    fn on(&mut self, arg: bool) {
        if arg {
            self.value.set(*self.on_value.get());
        }
    }

    #[mylife_action]
    fn off(&mut self, arg: bool) {
        if arg {
            self.value.set(*self.off_value.get());
        }
    }

    #[mylife_action]
    fn toggle(&mut self, arg: bool) {
        if arg {
            if *self.value.get() < *self.toggle_threshold.get() {
                self.value.set(*self.on_value.get());
            } else {
                self.value.set(*self.off_value.get());
            }
        }
    }

    #[mylife_action(r#type = "range[0;100]")]
    fn set_toggle_threshold(&mut self, arg: i64) {
        self.toggle_threshold.set(arg);
    }

    #[mylife_action(r#type = "range[0;100]")]
    fn set_on_value(&mut self, arg: i64) {
        let need_update = *self.value.get() == *self.on_value.get();
        self.on_value.set(arg);
        if need_update {
            self.value.set(*self.on_value.get());
        }
    }

    #[mylife_action(r#type = "range[0;100]")]
    fn set_off_value(&mut self, arg: i64) {
        let need_update = *self.value.get() == *self.off_value.get();
        self.off_value.set(arg);
        if need_update {
            self.value.set(*self.off_value.get());
        }
    }
}
