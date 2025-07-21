use serde::de::DeserializeOwned;
use serde_json::Value;
use std::error::Error;

use crate::utils::error_utils;

/// Extract and parse a field from a JSON Value, with a typed error if missing
///
/// # Arguments
///
/// * `value` - JSON value to extract from
/// * `field` - Field name to extract
///
/// # Returns
///
/// * `Result<T, Box<dyn Error>>` - Parsed field value or error
///
/// # Errors
///
/// Returns an error if the field is missing or cannot be parsed as type T.
pub fn extract_field<T: DeserializeOwned>(value: &Value, field: &str) -> Result<T, Box<dyn Error>> {
    value
        .get(field)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| error_utils::new_error(&format!("{field} missing or invalid in JSON")))
}

/// Extract a string field from a JSON Value
///
/// # Arguments
///
/// * `value` - JSON value to extract from
/// * `field` - Field name to extract
///
/// # Returns
///
/// * `Result<String, Box<dyn Error>>` - Field value as string or error
///
/// # Errors
///
/// Returns an error if the field is missing or not a string.
pub fn extract_string_field(value: &Value, field: &str) -> Result<String, Box<dyn Error>> {
    value
        .get(field)
        .and_then(|v| v.as_str())
        .map(std::string::ToString::to_string)
        .ok_or_else(|| error_utils::new_error(&format!("{field} missing or not a string")))
}

/// Extract a string field from a JSON Value with a fallback field name
///
/// # Arguments
///
/// * `value` - JSON value to extract from
/// * `field` - Primary field name to try
/// * `fallback` - Fallback field name if primary is missing
///
/// # Returns
///
/// * `Result<String, Box<dyn Error>>` - Field value as string or error
///
/// # Errors
///
/// Returns an error if both fields are missing or not strings.
pub fn extract_string_field_or(
    value: &Value,
    field: &str,
    fallback: &str,
) -> Result<String, Box<dyn Error>> {
    value
        .get(field)
        .and_then(|v| v.as_str())
        .or_else(|| value.get(fallback).and_then(|v| v.as_str()))
        .map(std::string::ToString::to_string)
        .ok_or_else(|| {
            error_utils::new_error(&format!(
                "Both {field} and {fallback} missing or not a string"
            ))
        })
}

/// Extract a number field from a JSON Value
///
/// # Arguments
///
/// * `value` - JSON value to extract from
/// * `field` - Field name to extract
///
/// # Returns
///
/// * `Result<T, Box<dyn Error>>` - Parsed number value or error
///
/// # Errors
///
/// Returns an error if the field is missing or cannot be parsed as the target type.
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
        .ok_or_else(|| error_utils::new_error(&format!("{field} missing or not a string")))
        .and_then(|s| {
            s.parse::<T>().map_err(|e| {
                error_utils::into_boxed_error(e, &format!("Failed to parse {field} as number"))
            })
        })
}
