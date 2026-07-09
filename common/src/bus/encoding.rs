use std::{error::Error, fmt, ops::RangeInclusive};

use bytes::Bytes;

use crate::components::{metadata::Type, types::Value};

#[derive(Debug)]
pub struct DecodingError;

impl Error for DecodingError {}

impl fmt::Display for DecodingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "could not decode value from buffer")
    }
}

/// Decode a string.
pub fn read_string(buffer: &Bytes) -> Result<String, DecodingError> {
    let str = str::from_utf8(buffer).map_err(|_| DecodingError)?;
    Ok(str.to_owned())
}

/// Encode a string.
pub fn write_string(value: &str) -> Bytes {
    Bytes::copy_from_slice(value.as_bytes())
}

/// Decode a boolean.
pub fn read_bool(buffer: &Bytes) -> Result<bool, DecodingError> {
    if buffer.is_empty() {
        return Err(DecodingError);
    }

    Ok(buffer[0] != 0)
}

/// Encode a boolean.
pub fn write_bool(value: bool) -> Bytes {
    let byte_value: u8 = if value { 1 } else { 0 };
    Bytes::from(vec![byte_value])
}

/// Decode an unsigned 8-bit integer.
pub fn read_u8(buffer: &Bytes) -> Result<u8, DecodingError> {
    if buffer.is_empty() {
        return Err(DecodingError);
    }

    Ok(buffer[0])
}

/// Encode an unsigned 8-bit integer.
pub fn write_u8(value: u8) -> Bytes {
    Bytes::from(vec![value])
}

/// Decode a signed 8-bit integer.
pub fn read_i8(buffer: &Bytes) -> Result<i8, DecodingError> {
    if buffer.is_empty() {
        return Err(DecodingError);
    }

    Ok(buffer[0] as i8)
}

/// Encode a signed 8-bit integer.
pub fn write_i8(value: i8) -> Bytes {
    Bytes::from(vec![value as u8])
}

/// Decode an unsigned 32-bit integer.
pub fn read_u32(buffer: &Bytes) -> Result<u32, DecodingError> {
    if buffer.len() < 4 {
        return Err(DecodingError);
    }

    Ok(u32::from_le_bytes(buffer[0..4].try_into().unwrap()))
}

/// Encode an unsigned 32-bit integer.
pub fn write_u32(value: u32) -> Bytes {
    Bytes::copy_from_slice(&value.to_le_bytes())
}

/// Decode a signed 32-bit integer.
pub fn read_i32(buffer: &Bytes) -> Result<i32, DecodingError> {
    if buffer.len() < 4 {
        return Err(DecodingError);
    }

    Ok(i32::from_le_bytes(buffer[0..4].try_into().unwrap()))
}

/// Encode a signed 32-bit integer.
pub fn write_i32(value: i32) -> Bytes {
    Bytes::copy_from_slice(&value.to_le_bytes())
}

/// Decode a 32-bit floating point number.
pub fn read_float(buffer: &Bytes) -> Result<f64, DecodingError> {
    if buffer.len() < 4 {
        return Err(DecodingError);
    }

    Ok(f32::from_le_bytes(buffer[0..4].try_into().unwrap()) as f64)
}

/// Encode a 32-bit floating point number.
pub fn write_float(value: f64) -> Bytes {
    Bytes::copy_from_slice(&(value as f32).to_le_bytes())
}

/// Decode a JSON value.
pub fn read_json(buffer: &Bytes) -> Result<serde_json::Value, DecodingError> {
    serde_json::from_slice(buffer).map_err(|_| DecodingError)
}

/// Encode a JSON value.
pub fn write_json(value: &serde_json::Value) -> Bytes {
    let json_string = serde_json::to_string(value).expect("Could not serialize JSON value");
    write_string(&json_string)
}

/// Decode a value based on its type.
pub fn read_value(ty: &Type, buffer: &Bytes) -> Result<Value, DecodingError> {
    match ty {
        Type::Range(range) => read_range(range, buffer).map(Value::Range),
        Type::Text => read_string(buffer).map(Value::Text),
        Type::Float => read_float(buffer).map(Value::Float),
        Type::Bool => read_bool(buffer).map(Value::Bool),
        Type::Enum(values) => read_enum(values, buffer).map(Value::Enum),
        Type::Complex => Err(DecodingError), // Not supported right now
    }
}

/// Encode a value based on its type.
pub fn write_value(ty: &Type, value: &Value) -> Bytes {
    match ty {
        Type::Range(range) => write_range(range, value.as_range().expect("Value is not a range")),
        Type::Text => write_string(value.as_text().expect("Value is not a text")),
        Type::Float => write_float(value.as_float().expect("Value is not a float")),
        Type::Bool => write_bool(value.as_bool().expect("Value is not a bool")),
        Type::Enum(values) => write_enum(values, value.as_enum().expect("Value is not an enum")),
        _ => panic!("Value type does not match the specified type"),
    }
}

fn read_range(range: &RangeInclusive<i64>, buffer: &Bytes) -> Result<i64, DecodingError> {
    if *range.start() >= 0 && *range.end() <= u8::MAX as i64 {
        return Ok(read_u8(buffer)? as i64);
    } else if *range.start() >= i8::MIN as i64 && *range.end() <= i8::MAX as i64 {
        return Ok(read_i8(buffer)? as i64);
    } else if *range.start() >= 0 && *range.end() <= u32::MAX as i64 {
        return Ok(read_u32(buffer)? as i64);
    } else if *range.start() >= i32::MIN as i64 && *range.end() <= i32::MAX as i64 {
        return Ok(read_i32(buffer)? as i64);
    }

    Err(DecodingError)
}

fn write_range(range: &RangeInclusive<i64>, value: i64) -> Bytes {
    if *range.start() >= 0 && *range.end() <= u8::MAX as i64 {
        return write_u8(value as u8);
    } else if *range.start() >= i8::MIN as i64 && *range.end() <= i8::MAX as i64 {
        return write_i8(value as i8);
    } else if *range.start() >= 0 && *range.end() <= u32::MAX as i64 {
        return write_u32(value as u32);
    } else if *range.start() >= i32::MIN as i64 && *range.end() <= i32::MAX as i64 {
        return write_i32(value as i32);
    }

    panic!(
        "Cannot represent range type with min={} and max={} because bounds are too big",
        range.start(),
        range.end()
    );
}

fn read_enum(values: &[String], buffer: &Bytes) -> Result<String, DecodingError> {
    let value = read_string(buffer)?;
    if !values.contains(&value) {
        return Err(DecodingError);
    }
    Ok(value)
}

fn write_enum(_values: &[String], value: &str) -> Bytes {
    write_string(value)
}
