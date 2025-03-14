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
    // Get image ID using podman
    let output = Command::new("podman")
        .args(["image", "inspect", "--format", "{{.Id}}", img])
        .output()
        .map_err(|e| PodmanHelperError::CommandExecution(format!("Failed to execute podman: {}", e)))?;

    if output.status.success() {
        // Parse the image ID
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| PodmanHelperError::OutputParsing(format!("Failed to parse podman output: {}", e)))?;
        
        let id = stdout.trim().to_string();

        // Get HOME directory safely
        let homedir = env::var("HOME")?;
        
        // Build path to manifest file
        let path = format!(
            "{}/.local/share/containers/storage/overlay-images/{}/manifest",
            homedir, id
        );
        
        // Run stat command on the manifest file
        let output2 = Command::new("stat")
            .args(["-c", "%y", &path])
            .output()
            .map_err(|e| PodmanHelperError::CommandExecution(format!("Failed to execute stat: {}", e)))?;

        if output2.status.success() {
            // Parse stat output
            let stdout2 = String::from_utf8(output2.stdout)
                .map_err(|e| PodmanHelperError::OutputParsing(format!("Failed to parse stat output: {}", e)))?;
                
            // Convert date string
            convert_str_to_date(stdout2.trim())
        } else {
            // Handle stat errors
            let stderr = String::from_utf8(output2.stderr)
                .map_err(|e| PodmanHelperError::OutputParsing(format!("Failed to parse stat output: {}", e)))?;
                
            Err(PodmanHelperError::CommandExecution(format!("stat failed: {}", stderr)))
        }
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
    
    let captures = re.captures(date_str).ok_or_else(|| 
        PodmanHelperError::DateParsing(format!("Failed to parse date from '{}'", date_str)))?;
    
    // Extract timezone offset
    let tz_offset = captures.name("tz_offset")
        .ok_or_else(|| PodmanHelperError::DateParsing(format!("Failed to parse timezone offset from '{}'", date_str)))?
        .as_str()
        .to_string();
    
    // Clean and prepare the date string
    let datetime_part = captures.name("datetime")
        .ok_or_else(|| PodmanHelperError::DateParsing(format!("Failed to parse datetime part from '{}'", date_str)))?
        .as_str();
        
    // Check if datetime part is valid
    if datetime_part.is_empty() {
        return Err(PodmanHelperError::DateParsing(format!("Empty datetime part in '{}'", date_str)));
    }
    
    // Replace T with space for consistency
    let cleaned_datetime = datetime_part.replace("T", " ");
    
    // Combine datetime with timezone offset
    let cleaned_date_str = if !tz_offset.is_empty() {
        format!("{}{}", cleaned_datetime, tz_offset)
    } else {
        format!("{}+0000", cleaned_datetime)
    };
    
    // Parse the cleaned string into a DateTime
    cleaned_date_str.parse::<DateTime<Utc>>()
        .map(|dt| dt.with_timezone(&Local))
        .map_err(|e| PodmanHelperError::DateParsing(format!("Failed to parse date '{}': {}", date_str, e)))
}
