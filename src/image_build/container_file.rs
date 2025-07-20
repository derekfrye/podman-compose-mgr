use ini::Ini;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContainerFileError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("INI parsing error: {0}")]
    IniParse(String),

    #[error("No Container section found in .container file")]
    NoContainerSection,

    #[error("No Image directive found in Container section")]
    NoImageDirective,

    #[error("Path not found: {0}")]
    PathNotFound(String),
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub image: String,
    pub name: Option<String>,
}

/// Parse a .container file and extract image information
///
/// # Arguments
///
/// * `file_path` - Path to the .container file
///
/// # Returns
///
/// * `Result<ContainerInfo, ContainerFileError>` - Container information or error
pub fn parse_container_file<P: AsRef<Path>>(file_path: P) -> Result<ContainerInfo, ContainerFileError> {
    let path = file_path.as_ref();
    
    // Load the INI file
    let conf = Ini::load_from_file(path)
        .map_err(|e| ContainerFileError::IniParse(e.to_string()))?;

    // Look for the Container section
    let container_section = conf.section(Some("Container"))
        .ok_or(ContainerFileError::NoContainerSection)?;

    // Extract the Image directive
    let image = container_section.get("Image")
        .ok_or(ContainerFileError::NoImageDirective)?
        .to_string();

    // Extract container name if present (from Unit section Description or filename)
    let name = extract_container_name(&conf, path);

    Ok(ContainerInfo {
        image,
        name,
    })
}

/// Extract container name from the .container file
///
/// This function looks for:
/// 1. ContainerName directive in [Container] section
/// 2. Description in [Unit] section
/// 3. Filename without .container extension as fallback
fn extract_container_name(conf: &Ini, file_path: &Path) -> Option<String> {
    // First, check for ContainerName in [Container] section
    if let Some(container_section) = conf.section(Some("Container")) {
        if let Some(container_name) = container_section.get("ContainerName") {
            return Some(container_name.to_string());
        }
    }

    // Second, check for Description in [Unit] section
    if let Some(unit_section) = conf.section(Some("Unit")) {
        if let Some(description) = unit_section.get("Description") {
            return Some(description.to_string());
        }
    }

    // Fallback: use filename without .container extension
    file_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_container_file_basic() {
        let content = r#"[Unit]
Description=My Test Container

[Container]
Image=docker.io/nginx:latest
PublishPort=8080:80

[Service]
Restart=always

[Install]
WantedBy=default.target
"#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), content).unwrap();

        let result = parse_container_file(temp_file.path()).unwrap();
        
        assert_eq!(result.image, "docker.io/nginx:latest");
        assert_eq!(result.name, Some("My Test Container".to_string()));
    }

    #[test]
    fn test_parse_container_file_with_container_name() {
        let content = r#"[Container]
Image=registry.example.com/myapp:v1.0
ContainerName=myapp-prod
PublishPort=3000:3000
"#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), content).unwrap();

        let result = parse_container_file(temp_file.path()).unwrap();
        
        assert_eq!(result.image, "registry.example.com/myapp:v1.0");
        assert_eq!(result.name, Some("myapp-prod".to_string()));
    }

    #[test]
    fn test_parse_container_file_filename_fallback() {
        let content = r#"[Container]
Image=alpine:latest
"#;

        let temp_file = NamedTempFile::with_suffix(".container").unwrap();
        fs::write(temp_file.path(), content).unwrap();

        let result = parse_container_file(temp_file.path()).unwrap();
        
        assert_eq!(result.image, "alpine:latest");
        // Should use filename without .container extension
        assert!(result.name.is_some());
    }

    #[test]
    fn test_parse_container_file_no_container_section() {
        let content = r#"[Unit]
Description=Invalid container file

[Service]
Type=simple
"#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), content).unwrap();

        let result = parse_container_file(temp_file.path());
        
        assert!(matches!(result, Err(ContainerFileError::NoContainerSection)));
    }

    #[test]
    fn test_parse_container_file_no_image() {
        let content = r#"[Container]
PublishPort=8080:80
"#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), content).unwrap();

        let result = parse_container_file(temp_file.path());
        
        assert!(matches!(result, Err(ContainerFileError::NoImageDirective)));
    }
}