use std::{collections::HashMap, fmt};

use crate::metadata;

pub trait MylifePluginRuntime {
    fn metadata(&self) -> &metadata::PluginMetadata;
    fn create(&self, id: &str) -> Box<dyn MylifeComponent>;
}

pub trait MylifeComponent {
    fn id(&self) -> &str;
    fn set_on_fail(&mut self, handler: Box<dyn Fn(/*error:*/ Box<dyn std::error::Error>)>);
    fn set_on_state(&mut self, handler: Box<dyn Fn(/*name:*/ &str, /*value:*/ Value)>);
    fn get_state(&self, name: &str) -> Result<Value, Box<dyn std::error::Error>>;
    fn configure(&mut self, config: &Config);
    fn init(&mut self);
    fn execute_action(&mut self, name: &str, action: Value);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RangePrimitive {
    U8,
    I8,
    U32,
    I32,
}

fn compute_range_primitive(min: i64, max: i64) -> RangePrimitive {
    if min >= u8::MIN.into() && max <= u8::MAX.into() {
        RangePrimitive::U8
    } else if min >= i8::MIN.into() && max <= i8::MAX.into() {
        RangePrimitive::I8
    } else if min >= u32::MIN.into() && max <= u32::MAX.into() {
        RangePrimitive::U32
    } else if min >= i32::MIN.into() && max <= i32::MAX.into() {
        RangePrimitive::I32
    } else {
        panic!(
            "Cannot represent range type with min={} and max={} because bounds are too big",
            min, max
        );
    }
}

impl TypedFrom<u8> for Value {
    fn typed_from(value: u8, ty: &metadata::Type) -> Self {
        if let metadata::Type::Range(min, max) = ty {
            if let RangePrimitive::U8 = compute_range_primitive(*min, *max) {
                return Value::RangeU8(value);
            }
        }

        panic!("Cannot convert from u8 to Value of type {:?}", ty);
    }
}

impl TypedFrom<i8> for Value {
    fn typed_from(value: i8, ty: &metadata::Type) -> Self {
        if let metadata::Type::Range(min, max) = ty {
            if let RangePrimitive::I8 = compute_range_primitive(*min, *max) {
                return Value::RangeI8(value);
            }
        }

        panic!("Cannot convert from i8 to Value of type {:?}", ty);
    }
}

impl TypedFrom<u32> for Value {
    fn typed_from(value: u32, ty: &metadata::Type) -> Self {
        if let metadata::Type::Range(min, max) = ty {
            if let RangePrimitive::U32 = compute_range_primitive(*min, *max) {
                return Value::RangeU32(value);
            }
        }

        panic!("Cannot convert from u32 to Value of type {:?}", ty);
    }
}

impl TypedFrom<i32> for Value {
    fn typed_from(value: i32, ty: &metadata::Type) -> Self {
        if let metadata::Type::Range(min, max) = ty {
            if let RangePrimitive::I32 = compute_range_primitive(*min, *max) {
                return Value::RangeI32(value);
            }
        }

        panic!("Cannot convert from i32 to Value of type {:?}", ty);
    }
}

impl TypedFrom<String> for Value {
    fn typed_from(value: String, ty: &metadata::Type) -> Self {
        match ty {
            metadata::Type::Text => Value::Text(value),
            metadata::Type::Enum(list) => {
                if !is_enum_member(list, &value) {
                    panic!(
                        "Unexpected enum value '{}'. (possibles values: [{}])",
                        value,
                        list.join(", ")
                    );
                }
                Value::Enum(value)
            }
            _ => panic!("Cannot convert from String to Value of type {:?}", ty),
        }
    }
}

fn is_enum_member(list: &Vec<String>, value: &String) -> bool {
    list.iter().any(|candidate| candidate == value)
}

impl TypedFrom<f32> for Value {
    fn typed_from(value: f32, ty: &metadata::Type) -> Self {
        if let metadata::Type::Float = ty {
            return Value::Float(value);
        }

        panic!("Cannot convert from f32 to Value of type {:?}", ty);
    }
}

impl TypedFrom<bool> for Value {
    fn typed_from(value: bool, ty: &metadata::Type) -> Self {
        if let metadata::Type::Bool = ty {
            return Value::Bool(value);
        }

        panic!("Cannot convert from bool to Value of type {:?}", ty);
    }
}

impl TypedTryFrom<Value> for u8 {
    type Error = ValueConversionError;

    fn typed_try_from(value: Value, ty: &metadata::Type) -> Result<Self, Self::Error> {
        if !is_range_primitive(ty, RangePrimitive::U8) {
            return Err(ValueConversionError::TypeMismatch(TypeMismatchData {
                native_type: "u8",
                ty: ty.clone(),
            }));
        }

        if let Value::RangeU8(value) = value {
            Ok(value)
        } else {
            Err(ValueConversionError::ValueMismatch(ValueMismatchData {
                native_type: "u8",
                ty: ty.clone(),
                value,
            }))
        }
    }
}

impl TypedTryFrom<Value> for i8 {
    type Error = ValueConversionError;

    fn typed_try_from(value: Value, ty: &metadata::Type) -> Result<Self, Self::Error> {
        if !is_range_primitive(ty, RangePrimitive::I8) {
            return Err(ValueConversionError::TypeMismatch(TypeMismatchData {
                native_type: "i8",
                ty: ty.clone(),
            }));
        }

        if let Value::RangeI8(value) = value {
            Ok(value)
        } else {
            Err(ValueConversionError::ValueMismatch(ValueMismatchData {
                native_type: "i8",
                ty: ty.clone(),
                value,
            }))
        }
    }
}

impl TypedTryFrom<Value> for u32 {
    type Error = ValueConversionError;

    fn typed_try_from(value: Value, ty: &metadata::Type) -> Result<Self, Self::Error> {
        if !is_range_primitive(ty, RangePrimitive::U32) {
            return Err(ValueConversionError::TypeMismatch(TypeMismatchData {
                native_type: "u32",
                ty: ty.clone(),
            }));
        }

        if let Value::RangeU32(value) = value {
            Ok(value)
        } else {
            Err(ValueConversionError::ValueMismatch(ValueMismatchData {
                native_type: "u32",
                ty: ty.clone(),
                value,
            }))
        }
    }
}

impl TypedTryFrom<Value> for i32 {
    type Error = ValueConversionError;

    fn typed_try_from(value: Value, ty: &metadata::Type) -> Result<Self, Self::Error> {
        if !is_range_primitive(ty, RangePrimitive::I32) {
            return Err(ValueConversionError::TypeMismatch(TypeMismatchData {
                native_type: "i32",
                ty: ty.clone(),
            }));
        }

        if let Value::RangeI32(value) = value {
            Ok(value)
        } else {
            Err(ValueConversionError::ValueMismatch(ValueMismatchData {
                native_type: "i32",
                ty: ty.clone(),
                value,
            }))
        }
    }
}

impl TypedTryFrom<Value> for String {
    type Error = ValueConversionError;

    fn typed_try_from(value: Value, ty: &metadata::Type) -> Result<Self, Self::Error> {
        match ty {
            metadata::Type::Enum(list) => {
                if let Value::Enum(value) = &value {
                    if is_enum_member(list, &value) {
                        return Ok(value.clone());
                    }
                }

                Err(ValueConversionError::ValueMismatch(ValueMismatchData {
                    native_type: "String",
                    ty: ty.clone(),
                    value,
                }))
            }
            metadata::Type::Text => {
                if let Value::Text(value) = value {
                    return Ok(value);
                }

                Err(ValueConversionError::ValueMismatch(ValueMismatchData {
                    native_type: "String",
                    ty: ty.clone(),
                    value,
                }))
            }

            _ => Err(ValueConversionError::TypeMismatch(TypeMismatchData {
                native_type: "String",
                ty: ty.clone(),
            })),
        }
    }
}

impl TypedTryFrom<Value> for f32 {
    type Error = ValueConversionError;

    fn typed_try_from(value: Value, ty: &metadata::Type) -> Result<Self, Self::Error> {
        if let metadata::Type::Float = ty {
        } else {
            return Err(ValueConversionError::TypeMismatch(TypeMismatchData {
                native_type: "f32",
                ty: ty.clone(),
            }));
        }

        if let Value::Float(value) = value {
            Ok(value)
        } else {
            Err(ValueConversionError::ValueMismatch(ValueMismatchData {
                native_type: "f32",
                ty: ty.clone(),
                value,
            }))
        }
    }
}

impl TypedTryFrom<Value> for bool {
    type Error = ValueConversionError;

    fn typed_try_from(value: Value, ty: &metadata::Type) -> Result<Self, Self::Error> {
        if let metadata::Type::Bool = ty {
        } else {
            return Err(ValueConversionError::TypeMismatch(TypeMismatchData {
                native_type: "bool",
                ty: ty.clone(),
            }));
        }

        if let Value::Bool(value) = value {
            Ok(value)
        } else {
            Err(ValueConversionError::ValueMismatch(ValueMismatchData {
                native_type: "bool",
                ty: ty.clone(),
                value,
            }))
        }
    }
}

fn is_range_primitive(ty: &metadata::Type, primitive: RangePrimitive) -> bool {
    if let metadata::Type::Range(min, max) = ty {
        compute_range_primitive(*min, *max) == primitive
    } else {
        false
    }
}

#[derive(Debug, Clone)]
pub enum ValueConversionError {
    TypeMismatch(TypeMismatchData),
    ValueMismatch(ValueMismatchData),
}

#[derive(Debug, Clone)]
pub struct TypeMismatchData {
    native_type: &'static str,
    ty: metadata::Type,
}

#[derive(Debug, Clone)]
pub struct ValueMismatchData {
    native_type: &'static str,
    ty: metadata::Type,
    value: Value,
}

impl fmt::Display for ValueConversionError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValueConversionError::TypeMismatch(data) => {
                write!(fmt, "Type mismatch: cannot convert {:?} into {}", data.ty, data.native_type)
            },
            ValueConversionError::ValueMismatch(data) => {
                write!(fmt, "Value mismatch: cannot convert value {:?} of type {:?} into {}", data.value, data.ty, data.native_type)
            },
        }
    }
}

impl std::error::Error for ValueConversionError {}

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
