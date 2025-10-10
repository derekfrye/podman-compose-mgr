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
    let (datetime_part, tz_offset) = capture_date_parts(date_str)?;

    if datetime_part.trim().is_empty() {
        return Err(DateTimeError::DateParsing(format!(
            "Empty datetime part in '{date_str}'"
        )));
    }

    let cleaned = normalize_datetime(&datetime_part, &tz_offset);
    parse_datetime(&cleaned, date_str)
}

fn capture_date_parts(date_str: &str) -> Result<(String, String), DateTimeError> {
    let regex = Regex::new(r"(?P<datetime>[0-9:\-\s\.T]+)(?P<tz_offset>[+-]\d{4})")?;
    let captures = regex.captures(date_str).ok_or_else(|| {
        DateTimeError::DateParsing(format!("Failed to parse date from '{date_str}'"))
    })?;

    let datetime_part = captures
        .name("datetime")
        .ok_or_else(|| {
            DateTimeError::DateParsing(format!("Failed to parse datetime part from '{date_str}'"))
        })?
        .as_str()
        .to_string();

    let tz_offset = captures
        .name("tz_offset")
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();

    Ok((datetime_part, tz_offset))
}

fn normalize_datetime(datetime_part: &str, tz_offset: &str) -> String {
    let cleaned = datetime_part.replace('T', " ");
    if tz_offset.is_empty() {
        format!("{cleaned}+0000")
    } else {
        format!("{cleaned}{tz_offset}")
    }
}

fn parse_datetime(cleaned: &str, original: &str) -> Result<DateTime<Local>, DateTimeError> {
    cleaned
        .parse::<DateTime<Utc>>()
        .map(|dt| dt.with_timezone(&Local))
        .map_err(|e| DateTimeError::DateParsing(format!("Failed to parse date '{original}': {e}")))
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
