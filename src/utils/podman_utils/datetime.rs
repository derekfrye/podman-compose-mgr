use chrono::{DateTime, Local, Utc};
use regex::Regex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DateTimeError {
    #[error("Date parsing error: {0}")]
    DateParsing(String),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
}

/// Convert a date string to a `DateTime` object
///
/// Handles various date formats returned by podman and stat commands
///
/// # Arguments
/// * `date_str` - The date string to parse
///
/// # Errors
/// Returns an error if:
/// - Failed to compile the regex
/// - Failed to find pattern matches in the date string
/// - Failed to parse the resulting date string into a `DateTime`
pub fn convert_str_to_date(date_str: &str) -> Result<DateTime<Local>, DateTimeError> {
    // Handle specific format returned by podman image inspect
    // Example: 2024-10-03 12:28:30.701255218 +0100 +0100

    // Extract the datetime and timezone components
    let re = Regex::new(r"(?P<datetime>[0-9:\-\s\.]+)(?P<tz_offset>[+-]\d{4})")?;

    let captures = re.captures(date_str).ok_or_else(|| {
        DateTimeError::DateParsing(format!("Failed to parse date from '{date_str}'"))
    })?;

    // Extract timezone offset
    let tz_offset = captures
        .name("tz_offset")
        .ok_or_else(|| {
            DateTimeError::DateParsing(format!("Failed to parse timezone offset from '{date_str}'"))
        })?
        .as_str()
        .to_string();

    // Clean and prepare the date string
    let datetime_part = captures
        .name("datetime")
        .ok_or_else(|| {
            DateTimeError::DateParsing(format!("Failed to parse datetime part from '{date_str}'"))
        })?
        .as_str();

    // Check if datetime part is valid
    if datetime_part.is_empty() {
        return Err(DateTimeError::DateParsing(format!(
            "Empty datetime part in '{date_str}'"
        )));
    }

    // Replace T with space for consistency
    let cleaned_datetime = datetime_part.replace('T', " ");

    // Combine datetime with timezone offset
    let cleaned_date_str = if tz_offset.is_empty() {
        format!("{cleaned_datetime}+0000")
    } else {
        format!("{cleaned_datetime}{tz_offset}")
    };

    // Parse the cleaned string into a DateTime
    cleaned_date_str
        .parse::<DateTime<Utc>>()
        .map(|dt| dt.with_timezone(&Local))
        .map_err(|e| DateTimeError::DateParsing(format!("Failed to parse date '{date_str}': {e}")))
}

/// Format a timestamp as a relative time (e.g., "5 minutes ago")
#[must_use]
pub fn format_time_ago(dt: DateTime<Local>) -> String {
    let now = Local::now();
    let duration = now.signed_duration_since(dt);
    let days = duration.num_days();
    let hours = duration.num_hours();
    let minutes = duration.num_minutes();
    let seconds = duration.num_seconds();

    if days > 0 {
        format!("{days} days ago")
    } else if hours > 0 {
        format!("{hours} hours ago")
    } else if minutes > 0 {
        format!("{minutes} minutes ago")
    } else {
        format!("{seconds} seconds ago")
    }
}
