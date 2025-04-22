use chrono::{DateTime, Local, TimeZone, Utc};
use regex::Regex;
use std::env;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PodmanHelperError {
    #[error("Command execution error: {0}")]
    CommandExecution(String),
    
    #[error("Output parsing error: {0}")]
    OutputParsing(String),
    
    #[error("Date parsing error: {0}")]
    DateParsing(String),
    
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    
    #[error("Environment variable error: {0}")]
    EnvVarError(#[from] env::VarError),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

pub type Result<T> = std::result::Result<T, PodmanHelperError>;

/// Get the upstream creation time of a podman image
///
/// # Arguments
/// * `img` - The image name to inspect
///
/// # Errors
/// Returns an error if:
/// - Failed to execute podman command
/// - Failed to parse the output
/// - Failed to parse the date
pub fn get_podman_image_upstream_create_time(img: &str) -> Result<DateTime<Local>> {
    let output = Command::new("podman")
        .args(["image", "inspect", "--format", "{{.Created}}", img])
        .output()
        .map_err(|e| PodmanHelperError::CommandExecution(format!("Failed to execute podman: {}", e)))?;

    if output.status.success() {
        // Parse stdout into a string
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| PodmanHelperError::OutputParsing(format!("Failed to parse podman output: {}", e)))?;
            
        // Convert the date string
        convert_str_to_date(stdout.trim())
    } else {
        // Handle "image not known" error gracefully
        match std::str::from_utf8(&output.stderr) {
            Ok(stderr_str) if stderr_str.contains("image not known") => {
                // Return a placeholder date for unknown images
                let dt = Local.with_ymd_and_hms(1900, 1, 1, 0, 0, 0)
                    .single()
                    .ok_or_else(|| PodmanHelperError::DateParsing("Failed to create placeholder date".to_string()))?;
                Ok(dt)
            },
            Ok(stderr_str) => {
                Err(PodmanHelperError::CommandExecution(format!("podman failed: {}", stderr_str)))
            },
            Err(e) => {
                Err(PodmanHelperError::OutputParsing(format!("Failed to parse stderr as UTF-8: {}", e)))
            }
        }
    }
}

/// Get image ID using podman command
fn get_image_id(img: &str) -> Result<String> {
    let output = Command::new("podman")
        .args(["image", "inspect", "--format", "{{.Id}}", img])
        .output()
        .map_err(|e| PodmanHelperError::CommandExecution(format!("Failed to execute podman: {}", e)))?;

    if output.status.success() {
        // Parse the image ID
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| PodmanHelperError::OutputParsing(format!("Failed to parse podman output: {}", e)))?;
        
        Ok(stdout.trim().to_string())
    } else {
        // Handle error
        handle_image_not_known_error(&output.stderr)
    }
}

/// Handle "image not known" error gracefully
fn handle_image_not_known_error(stderr: &[u8]) -> Result<String> {
    match std::str::from_utf8(stderr) {
        Ok(stderr_str) if stderr_str.contains("image not known") => {
            // Return empty string to signal a placeholder date should be used
            Ok(String::new())
        },
        Ok(stderr_str) => {
            Err(PodmanHelperError::CommandExecution(format!("podman failed: {}", stderr_str)))
        },
        Err(e) => {
            Err(PodmanHelperError::OutputParsing(format!("Failed to parse stderr as UTF-8: {}", e)))
        }
    }
}

/// Create a placeholder date for unknown images
fn create_placeholder_date() -> Result<DateTime<Local>> {
    Local.with_ymd_and_hms(1900, 1, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| PodmanHelperError::DateParsing("Failed to create placeholder date".to_string()))
}

/// Get stat command output for a file
fn get_file_stat(path: &str) -> Result<String> {
    let output = Command::new("stat")
        .args(["-c", "%y", path])
        .output()
        .map_err(|e| PodmanHelperError::CommandExecution(format!("Failed to execute stat: {}", e)))?;

    if output.status.success() {
        // Parse stat output
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| PodmanHelperError::OutputParsing(format!("Failed to parse stat output: {}", e)))?;
            
        Ok(stdout.trim().to_string())
    } else {
        // Handle stat errors
        let stderr = String::from_utf8(output.stderr)
            .map_err(|e| PodmanHelperError::OutputParsing(format!("Failed to parse stat output: {}", e)))?;
            
        Err(PodmanHelperError::CommandExecution(format!("stat failed: {}", stderr)))
    }
}

/// Get the local disk modification time of a podman image
///
/// # Arguments
/// * `img` - The image name to inspect
///
/// # Errors
/// Returns an error if:
/// - Failed to execute podman command
/// - Failed to parse the output
/// - Failed to get the HOME environment variable
/// - Failed to execute stat command
/// - Failed to parse the date
pub fn get_podman_ondisk_modify_time(img: &str) -> Result<DateTime<Local>> {
    // Get image ID
    let id = get_image_id(img)?;
    
    // Handle empty ID (image not found)
    if id.is_empty() {
        return create_placeholder_date();
    }
    
    // Get HOME directory safely
    let homedir = env::var("HOME")?;
    
    // Build path to manifest file
    let path = format!(
        "{}/.local/share/containers/storage/overlay-images/{}/manifest",
        homedir, id
    );
    
    // Get file modification time
    let date_str = get_file_stat(&path)?;
    
    // Convert date string
    convert_str_to_date(&date_str)
}

/// Extract datetime part from date string
fn extract_datetime_part(date_str: &str, re: &Regex) -> Result<String> {
    let captures = re.captures(date_str).ok_or_else(|| 
        PodmanHelperError::DateParsing(format!("Failed to parse date from '{}'", date_str)))?;
    
    let datetime_part = captures.name("datetime")
        .ok_or_else(|| PodmanHelperError::DateParsing(format!("Failed to parse datetime part from '{}'", date_str)))?
        .as_str();
        
    // Check if datetime part is valid
    if datetime_part.is_empty() {
        return Err(PodmanHelperError::DateParsing(format!("Empty datetime part in '{}'", date_str)));
    }
    
    Ok(datetime_part.to_string())
}

/// Extract timezone offset from date string
fn extract_timezone_offset(date_str: &str, re: &Regex) -> Result<String> {
    let captures = re.captures(date_str).ok_or_else(|| 
        PodmanHelperError::DateParsing(format!("Failed to parse date from '{}'", date_str)))?;
    
    let tz_offset = captures.name("tz_offset")
        .ok_or_else(|| PodmanHelperError::DateParsing(format!("Failed to parse timezone offset from '{}'", date_str)))?
        .as_str()
        .to_string();
    
    Ok(tz_offset)
}

/// Parse cleaned date string to DateTime object
fn parse_datetime(cleaned_date_str: &str, original_date_str: &str) -> Result<DateTime<Local>> {
    cleaned_date_str.parse::<DateTime<Utc>>()
        .map(|dt| dt.with_timezone(&Local))
        .map_err(|e| PodmanHelperError::DateParsing(format!("Failed to parse date '{}': {}", original_date_str, e)))
}

/// Convert a date string to a DateTime object
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
/// - Failed to parse the resulting date string into a DateTime
fn convert_str_to_date(date_str: &str) -> Result<DateTime<Local>> {
    // Handle specific format returned by podman image inspect
    // Example: 2024-10-03 12:28:30.701255218 +0100 +0100
    
    // Extract the datetime and timezone components
    let re = Regex::new(r"(?P<datetime>[0-9:\-\s\.]+)(?P<tz_offset>[+-]\d{4})")?;
    
    // Extract datetime and timezone parts
    let datetime_part = extract_datetime_part(date_str, &re)?;
    let tz_offset = extract_timezone_offset(date_str, &re)?;
    
    // Replace T with space for consistency
    let cleaned_datetime = datetime_part.replace("T", " ");
    
    // Combine datetime with timezone offset
    let cleaned_date_str = format!("{}{}", cleaned_datetime, tz_offset);
    
    // Parse the cleaned string into a DateTime
    parse_datetime(&cleaned_date_str, date_str)
}