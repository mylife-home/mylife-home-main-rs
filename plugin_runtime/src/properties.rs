use std::ops::Deref;

#[derive(Debug)]
pub enum NetValue {
    Int8(i8),
    UInt8(i8),
    Int32(i32),
    UInt32(i32),
    String(String),
    Float(f64),
    Bool(bool),
    Complex(),
}

#[derive(Debug)]
pub enum ConfigValue {
    String(String),
    Bool(bool),
    Integer(i64),
    Float(f64),
}

pub trait StateDef {
    fn runtime_register(&mut self, handler: fn(value: &NetValue));
    fn runtime_get(&self) -> &NetValue;
}

pub struct State<T> {
    value: T,
    handler: Option<fn(value: NetValue)>,
}

impl<T> State<T> {
    pub fn change(&mut self, value: T) {
        self.value = value;

        if let Some(handler) = self.handler {
            handler(&self.value);
        }
    }
}

impl<T> StateDef for State<T> {
    fn runtime_register(&mut self, handler: fn(value: &NetValue)) {
        self.handler = Some(handler);
    }

    fn runtime_get(&self) -> &NetValue {
        &self.value
    }
}

impl<T> Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

// TODO: improve
pub enum ConfigError {
    InvalidType,
}

pub trait ConfigDef {
    fn runtime_init(&mut self, value: ConfigValue) -> Result<(), ConfigError>;
}

pub struct Config<T> {
    value: T,
}

impl<T> ConfigDef for Config<T> {
    fn runtime_init(&mut self, value: ConfigValue) -> Result<(), ConfigError> {
        // TODO: check type
        self.value = value;

        Ok(())
    }
}

impl<T> Deref for Config<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

pub enum ActionError {
    NotBound,
}

pub trait ActionDef {
    fn runtime_validate(&self) -> Result<(), ActionError>;
    fn runtime_set(&self, value: NetValue);
}

pub struct Action<T> {
    handler: Option<fn(value: T)>,
}

impl<T> Action<T> {
    pub fn bind(&mut self, handler: fn(value: T)) {
        self.handler = Some(handler);
    }
}

impl<T> ActionDef for Action<T> {
    fn runtime_validate(&self) -> Result<(), ActionError> {
        if let None = self.handler {
            Err(ActionError::NotBound)
        } else {
            Ok(())
        }
    }

    fn runtime_set(&self, value: NetValue) {
        let handler = self.handler.as_ref().unwrap();
        handler(value);
    }
}
