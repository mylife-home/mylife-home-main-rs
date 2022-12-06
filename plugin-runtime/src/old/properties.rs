pub mod state {
    use crate::NetValue;
    use std::ops::Deref;

    pub trait Definition {
        fn runtime_register(&mut self, handler: fn(value: &NetValue));
        fn runtime_get(&self) -> &NetValue;
    }

    pub struct Int8 {
        value: i8,
        handler: Option<fn(value: NetValue)>,
    }

    impl Int8 {
        pub fn change(&mut self, value: i8) {
            self.value = value;

            if let Some(handler) = self.handler {
                handler(NetValue::Int8(self.value));
            }
        }
    }

    impl Definition for Int8 {
        fn runtime_register(&mut self, handler: fn(value: NetValue)) {
            self.handler = Some(handler);
        }

        fn runtime_get(&self) -> NetValue {
            NetValue::Int8(self.value)
        }
    }

    impl<T> Deref for Int8 {
        type Target = i8;

        fn deref(&self) -> &Self::Target {
            &self.value
        }
    }
}

pub mod config {
    use crate::ConfigValue;
    use std::ops::Deref;

    // TODO: improve
    pub enum ConfigError {
        InvalidType,
    }

    pub trait Definition {
        fn runtime_init(&mut self, value: ConfigValue) -> Result<(), ConfigError>;
    }

    pub struct Integer {
        value: i64,
    }

    impl Definition for Integer {
        fn runtime_init(&mut self, value: ConfigValue) -> Result<(), ConfigError> {
            // TODO: check type
            self.value = value;

            Ok(())
        }
    }

    impl Deref for Integer {
        type Target = i64;

        fn deref(&self) -> &Self::Target {
            &self.value
        }
    }
}

pub mod action {
    use crate::NetValue;

    pub enum ActionError {
        NotBound,
    }

    pub trait Definition {
        fn runtime_validate(&self) -> Result<(), ActionError>;
        fn runtime_set(&self, value: NetValue);
    }

    pub struct Int8 {
        handler: Option<fn(value: i8)>,
    }

    impl Int8 {
        pub fn bind(&mut self, handler: fn(value: i8)) {
            self.handler = Some(handler);
        }
    }

    impl Definition for Int8 {
        fn runtime_validate(&self) -> Result<(), ActionError> {
            if let None = self.handler {
                Err(ActionError::NotBound)
            } else {
                Ok(())
            }
        }

        fn runtime_set(&self, value: NetValue) {
            if let NetValue::Int8(typed_value) = value {
                let handler = self.handler.as_ref().unwrap();
                handler(typed_value);
            }
        }
    }
}
