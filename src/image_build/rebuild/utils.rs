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
