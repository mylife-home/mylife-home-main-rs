use std::{collections::HashMap, fmt};

pub type Config = HashMap<String, ConfigValue>;

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

impl std::error::Error for ConfigValueConversionError {}
