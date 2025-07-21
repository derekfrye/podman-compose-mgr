use crate::image_build::rebuild_error::RebuildError;
use serde_yaml::Mapping;
use serde_yaml::Value;
use std::fs::File;

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

/// Extract services map from YAML
///
/// # Errors
///
/// Returns an error if:
/// - 'services' section is missing
/// - 'services' is not a mapping
pub fn extract_services(yaml: &Value) -> Result<Mapping, RebuildError> {
    let services = yaml.get("services").ok_or_else(|| {
        RebuildError::MissingField("No 'services' section found in compose file".to_string())
    })?;

    services
        .as_mapping()
        .ok_or_else(|| RebuildError::InvalidConfig("'services' is not a mapping".to_string()))
        .cloned()
}

/// Extract image and container name from service config
///
/// # Errors
///
/// Returns an error if:
/// - 'image' or '`container_name`' is not a string
pub fn extract_image_info(
    service_config: &Value,
) -> Result<Option<(String, String)>, RebuildError> {
    // Get image name if present
    if let Some(image) = service_config.get("image") {
        // Get container name if present
        if let Some(container_name) = service_config.get("container_name") {
            // Extract string values safely
            let image_string = image
                .as_str()
                .ok_or_else(|| RebuildError::InvalidConfig("'image' is not a string".to_string()))?
                .to_string();

            let container_nm_string = container_name
                .as_str()
                .ok_or_else(|| {
                    RebuildError::InvalidConfig("'container_name' is not a string".to_string())
                })?
                .to_string();

            return Ok(Some((image_string, container_nm_string)));
        }
    }

    Ok(None)
}
