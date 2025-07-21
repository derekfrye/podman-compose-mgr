use chrono::{DateTime, Local};
use serde_yaml::Value;
use std::fs::File;

use super::errors::RebuildError;

/// Read and parse a YAML file
///
/// # Errors
///
/// Returns an error if:
/// - Unable to open the file
/// - Unable to parse the file as YAML
pub fn read_yaml_file(file_path: &str) -> Result<Value, RebuildError> {
    // Open the file
    let file = File::open(file_path).map_err(RebuildError::Io)?;

    // Parse as YAML
    let yaml: Value = serde_yaml::from_reader(file).map_err(RebuildError::YamlParse)?;

    Ok(yaml)
}

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
