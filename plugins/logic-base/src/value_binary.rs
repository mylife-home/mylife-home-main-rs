#[derive(MylifeComponent)]
#[mylife_component(description = "step relay")] // name=
pub struct ValueBinary {
  #[mylife_config(description="useless")] // type=, name=
  config: bool,

  #[mylife_state(description= "actual value")] // type=, name=
  state: State<bool> 
}

#[mylife_actions]
impl ValueBinary {
  #[mylife_action(description= "set value to on")] // type=, name=
  fn on(arg: bool) {
    if (arg) {
      this.state.set(true);
    }
  }

  #[mylife_action(description= "set value to off")]
  fn off(arg: bool) {
    if (arg) {
      this.state.set(false);
    }
  }

  #[mylife_action(description= "toggle value")]
  fn toggle(arg: bool) {
    if (arg) {
      this.state.set(!this.state.get());
    }
  }
}
