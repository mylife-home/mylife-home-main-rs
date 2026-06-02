use std::{error::Error, fmt};

use bytes::Bytes;

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

/// Decode an integer.
pub fn read_i64(buffer: &Bytes) -> Result<i64, DecodingError> {
    if buffer.len() < 8 {
        return Err(DecodingError);
    }

    let buff = buffer[0..8].try_into().map_err(|_| DecodingError)?;
    Ok(i64::from_le_bytes(buff))
}

/// Encode an integer.
pub fn write_i64(value: i64) -> Bytes {
    Bytes::copy_from_slice(&value.to_le_bytes())
}

/// Decode a float.
pub fn read_f64(buffer: &Bytes) -> Result<f64, DecodingError> {
    if buffer.len() < 8 {
        return Err(DecodingError);
    }

    let buff = buffer[0..8].try_into().map_err(|_| DecodingError)?;
    Ok(f64::from_le_bytes(buff))
}

/// Encode a float.
pub fn write_f64(value: f64) -> Bytes {
    Bytes::copy_from_slice(&value.to_le_bytes())
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
