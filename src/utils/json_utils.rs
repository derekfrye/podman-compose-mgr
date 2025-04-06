use serde::de::DeserializeOwned;
use serde_json::Value;
use std::error::Error;

use crate::utils::error_utils;

/// Extract and parse a field from a JSON Value, with a typed error if missing
pub fn extract_field<T: DeserializeOwned>(value: &Value, field: &str) -> Result<T, Box<dyn Error>> {
    value
        .get(field)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| error_utils::new_error(&format!("{} missing or invalid in JSON", field)))
}

/// Extract a string field from a JSON Value
pub fn extract_string_field(value: &Value, field: &str) -> Result<String, Box<dyn Error>> {
    value
        .get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| error_utils::new_error(&format!("{} missing or not a string", field)))
}

/// Extract a string field from a JSON Value with a fallback field name
pub fn extract_string_field_or(value: &Value, field: &str, fallback: &str) -> Result<String, Box<dyn Error>> {
    value
        .get(field)
        .and_then(|v| v.as_str())
        .or_else(|| value.get(fallback).and_then(|v| v.as_str()))
        .map(|s| s.to_string())
        .ok_or_else(|| error_utils::new_error(&format!("Both {} and {} missing or not a string", field, fallback)))
}

/// Extract a number field from a JSON Value
pub fn extract_number_field<T: std::str::FromStr>(
    value: &Value,
    field: &str,
) -> Result<T, Box<dyn Error>>
where
    <T as std::str::FromStr>::Err: Error + 'static,
{
    value
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_utils::new_error(&format!("{} missing or not a string", field)))
        .and_then(|s| {
            s.parse::<T>().map_err(|e| {
                error_utils::into_boxed_error(e, &format!("Failed to parse {} as number", field))
            })
        })
}
