pub mod metadata;

use std::ops::Deref;

pub trait Plugin {
  fn init(&mut self) {
  }

  fn terminate(&mut self) {
  }
}

pub struct State<T> {
  value: T,
  // callbacks
}

impl<T> State<T> {
  pub fn change(&mut self, value: T) {
    self.value = value;
    // callbacks
  }
}

impl<T> Deref for State<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
      &self.value
  }
}

pub struct Config<T> {
  value: T,
  // init
}

impl<T> Deref for Config<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
      &self.value
  }
}

pub struct Action<T> {
  value: T,
  // handler
}
