use std::{collections::HashMap, fmt};

use crate::metadata;

pub trait MylifePluginRuntime {
    fn metadata(&self) -> &metadata::PluginMetadata;
    fn create(&self) -> Box<dyn MylifeComponent>;
}

pub trait MylifeComponent {
    fn set_on_fail(&mut self, handler: Box<dyn Fn(/*error:*/ Box<dyn std::error::Error>)>);
    fn set_on_state(&mut self, handler: Box<dyn Fn(/*name:*/ &str, /*state:*/ Value)>);
    fn configure(&mut self, config: &Config);
    fn execute_action(&mut self, name: &str, action: &Value);
}

pub type Config = HashMap<String, ConfigValue>;

#[derive(Debug, Clone)]
pub enum Value {
    RangeU8(u8),
    RangeI8(i8),
    RangeU32(u32),
    RangeI32(i32),
    Text(String),
    Float(f32),
    Bool(bool),
    Enum(String), // TODO: native enum binding?
    Complex,      // unsupported for now
}

pub trait TypedFrom<T>: Sized {
    fn typed_from(value: T, ty: &metadata::Type) -> Self;
}

pub trait TypedInto<T>: Sized {
    fn typed_into(self, ty: &metadata::Type) -> T;
}

impl<T, U> TypedInto<U> for T
where
    U: TypedFrom<T>,
{
    fn typed_into(self, ty: &metadata::Type) -> U {
        U::typed_from(self, ty)
    }
}

pub trait TypedTryFrom<T>: Sized {
    type Error;

    fn typed_try_from(value: T, ty: &metadata::Type) -> Result<Self, Self::Error>;
}

pub trait TypedTryInto<T>: Sized {
    type Error;

    fn typed_try_into(self, ty: &metadata::Type) -> Result<T, Self::Error>;
}

impl<T, U> TypedTryInto<U> for T
where
    U: TypedTryFrom<T>,
{
    type Error = U::Error;

    fn typed_try_into(self, ty: &metadata::Type) -> Result<U, U::Error> {
        U::typed_try_from(self, ty)
    }
}

impl TypedFrom<u8> for Value {
    fn typed_from(value: u8, ty: &metadata::Type) -> Self {
        todo!()
    }
}

impl TypedFrom<i8> for Value {
    fn typed_from(value: i8, ty: &metadata::Type) -> Self {
        todo!()
    }
}

impl TypedFrom<u32> for Value {
    fn typed_from(value: u32, ty: &metadata::Type) -> Self {
        todo!()
    }
}

impl TypedFrom<i32> for Value {
    fn typed_from(value: i32, ty: &metadata::Type) -> Self {
        todo!()
    }
}

impl TypedFrom<String> for Value {
    fn typed_from(value: String, ty: &metadata::Type) -> Self {
        // Text + Enum
        todo!()
    }
}

impl TypedFrom<f32> for Value {
    fn typed_from(value: f32, ty: &metadata::Type) -> Self {
        todo!()
    }
}

impl TypedFrom<bool> for Value {
    fn typed_from(value: bool, ty: &metadata::Type) -> Self {
        todo!()
    }
}

impl TypedTryFrom<Value> for u8 {
    type Error = ValueConversionError;

    fn typed_try_from(value: Value, ty: &metadata::Type) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TypedTryFrom<Value> for i8 {
    type Error = ValueConversionError;

    fn typed_try_from(value: Value, ty: &metadata::Type) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TypedTryFrom<Value> for u32 {
    type Error = ValueConversionError;

    fn typed_try_from(value: Value, ty: &metadata::Type) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TypedTryFrom<Value> for i32 {
    type Error = ValueConversionError;

    fn typed_try_from(value: Value, ty: &metadata::Type) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TypedTryFrom<Value> for String {
    type Error = ValueConversionError;

    fn typed_try_from(value: Value, ty: &metadata::Type) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TypedTryFrom<Value> for f32 {
    type Error = ValueConversionError;

    fn typed_try_from(value: Value, ty: &metadata::Type) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TypedTryFrom<Value> for bool {
    type Error = ValueConversionError;

    fn typed_try_from(value: Value, ty: &metadata::Type) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct ValueConversionError();

impl fmt::Display for ValueConversionError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        todo!()
    }
}

impl std::error::Error for ValueConversionError {

}

#[derive(Debug, Clone)]
pub enum ConfigValue {
    String(String),
    Bool(bool),
    Integer(i64),
    Float(f64),
}

impl From<String> for ConfigValue {
    fn from(value: String) -> Self {
        ConfigValue::String(value)
    }
}

impl From<bool> for ConfigValue {
    fn from(value: bool) -> Self {
        ConfigValue::Bool(value)
    }
}

impl From<i64> for ConfigValue {
    fn from(value: i64) -> Self {
        ConfigValue::Integer(value)
    }
}

impl From<f64> for ConfigValue {
    fn from(value: f64) -> Self {
        ConfigValue::Float(value)
    }
}

impl TryFrom<ConfigValue> for String {
    type Error = ConfigValueConversionError;

    fn try_from(value: ConfigValue) -> Result<Self, Self::Error> {
        if let ConfigValue::String(value) = value {
            Ok(value)
        } else {
            Err(ConfigValueConversionError {
                expected: ConfigValue::String(String::default()),
                actual: value,
            })
        }
    }
}

impl TryFrom<ConfigValue> for bool {
    type Error = ConfigValueConversionError;

    fn try_from(value: ConfigValue) -> Result<Self, Self::Error> {
        if let ConfigValue::Bool(value) = value {
            Ok(value)
        } else {
            Err(ConfigValueConversionError {
                expected: ConfigValue::Bool(bool::default()),
                actual: value,
            })
        }
    }
}

impl TryFrom<ConfigValue> for i64 {
    type Error = ConfigValueConversionError;

    fn try_from(value: ConfigValue) -> Result<Self, Self::Error> {
        if let ConfigValue::Integer(value) = value {
            Ok(value)
        } else {
            Err(ConfigValueConversionError {
                expected: ConfigValue::Integer(i64::default()),
                actual: value,
            })
        }
    }
}

impl TryFrom<ConfigValue> for f64 {
    type Error = ConfigValueConversionError;

    fn try_from(value: ConfigValue) -> Result<Self, Self::Error> {
        if let ConfigValue::Float(value) = value {
            Ok(value)
        } else {
            Err(ConfigValueConversionError {
                expected: ConfigValue::Float(f64::default()),
                actual: value,
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigValueConversionError {
    expected: ConfigValue,
    actual: ConfigValue,
}

impl fmt::Display for ConfigValueConversionError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let expected = match self.expected {
            ConfigValue::String(_) => "String",
            ConfigValue::Bool(_) => "Bool",
            ConfigValue::Integer(_) => "Integer",
            ConfigValue::Float(_) => "Float",
        };

        let actual = match &self.actual {
            ConfigValue::String(value) => format!("String('{}')", value),
            ConfigValue::Bool(value) => format!("Bool({})", value),
            ConfigValue::Integer(value) => format!("Integer({})", value),
            ConfigValue::Float(value) => format!("Float({})", value),
        };

        write!(
            fmt,
            "Could not convert config value (expected type: {}, actual value: {}",
            expected, actual
        )
    }
}

impl std::error::Error for ConfigValueConversionError {
    
}
