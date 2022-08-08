#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

pub trait Plugin {
  fn init(&mut self) {
  }

  fn terminate(&mut self) {
  }
}

pub struct State<T> {
  value: T;
  // callbacks
}

impl<T> State<T> {
  pub fn change(&mut self, value: T) {
    &self.value = value;
    // callbacks
  }
}

impl<T> Deref for State<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
      &self.value
  }
}