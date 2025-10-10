use super::datetime::convert_str_to_date;
use chrono::{DateTime, Local, TimeZone};
use dockerfile_parser::Dockerfile;
use std::env;
use std::io::{BufReader, Read};
use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImageError {
    #[error("Command execution error: {0}")]
    CommandExecution(String),

    #[error("Output parsing error: {0}")]
    OutputParsing(String),

    #[error("Date parsing error: {0}")]
    DateParsing(String),

    #[error("Environment variable error: {0}")]
    EnvVarError(#[from] env::VarError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

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
pub fn get_podman_image_upstream_create_time(img: &str) -> Result<DateTime<Local>, ImageError> {
    let output = Command::new("podman")
        .args(["image", "inspect", "--format", "{{.Created}}", img])
        .output()
        .map_err(|e| ImageError::CommandExecution(format!("Failed to execute podman: {e}")))?;

    if output.status.success() {
        // Parse stdout into a string
        let stdout = String::from_utf8(output.stdout).map_err(|e| {
            ImageError::OutputParsing(format!("Failed to parse podman output: {e}"))
        })?;

        // Convert the date string
        convert_str_to_date(stdout.trim()).map_err(|e| ImageError::DateParsing(e.to_string()))
    } else {
        // Handle "image not known" error gracefully
        match std::str::from_utf8(&output.stderr) {
            Ok(stderr_str) if stderr_str.contains("image not known") => {
                // Return a placeholder date for unknown images
                let dt = Local
                    .with_ymd_and_hms(1900, 1, 1, 0, 0, 0)
                    .single()
                    .ok_or_else(|| {
                        ImageError::DateParsing("Failed to create placeholder date".to_string())
                    })?;
                Ok(dt)
            }
            Ok(stderr_str) => Err(ImageError::CommandExecution(format!(
                "podman failed: {stderr_str}"
            ))),
            Err(e) => Err(ImageError::OutputParsing(format!(
                "Failed to parse stderr as UTF-8: {e}"
            ))),
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
pub fn get_podman_ondisk_modify_time(img: &str) -> Result<DateTime<Local>, ImageError> {
    let Some(image_id) = inspect_image_id(img)? else {
        return placeholder_date();
    };

    let manifest_path = build_manifest_path(&image_id)?;
    let stat_output = stat_manifest(&manifest_path)?;
    convert_str_to_date(stat_output.trim()).map_err(|e| ImageError::DateParsing(e.to_string()))
}

fn inspect_image_id(img: &str) -> Result<Option<String>, ImageError> {
    let output = Command::new("podman")
        .args(["image", "inspect", "--format", "{{.Id}}", img])
        .output()
        .map_err(|e| ImageError::CommandExecution(format!("Failed to execute podman: {e}")))?;

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).map_err(|e| {
            ImageError::OutputParsing(format!("Failed to parse podman output: {e}"))
        })?;
        Ok(Some(stdout.trim().to_string()))
    } else {
        handle_unknown_image(&output)
    }
}

fn build_manifest_path(image_id: &str) -> Result<String, ImageError> {
    let homedir = env::var("HOME")?;
    Ok(format!(
        "{homedir}/.local/share/containers/storage/overlay-images/{image_id}/manifest"
    ))
}

fn stat_manifest(path: &str) -> Result<String, ImageError> {
    let output = Command::new("stat")
        .args(["-c", "%y", path])
        .output()
        .map_err(|e| ImageError::CommandExecution(format!("Failed to execute stat: {e}")))?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|e| ImageError::OutputParsing(format!("Failed to parse stat output: {e}")))
    } else {
        let stderr = String::from_utf8(output.stderr)
            .map_err(|e| ImageError::OutputParsing(format!("Failed to parse stat output: {e}")))?;

        Err(ImageError::CommandExecution(format!(
            "stat failed: {stderr}"
        )))
    }
}

fn handle_unknown_image(output: &std::process::Output) -> Result<Option<String>, ImageError> {
    match std::str::from_utf8(&output.stderr) {
        Ok(stderr_str) if stderr_str.contains("image not known") => Ok(None),
        Ok(stderr_str) => Err(ImageError::CommandExecution(format!(
            "podman failed: {stderr_str}"
        ))),
        Err(e) => Err(ImageError::OutputParsing(format!(
            "Failed to parse stderr as UTF-8: {e}"
        ))),
    }
}

fn placeholder_date() -> Result<DateTime<Local>, ImageError> {
    Local
        .with_ymd_and_hms(1900, 1, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| ImageError::DateParsing("Failed to create placeholder date".to_string()))
}

/// Parse Dockerfile and pull base image
///
/// # Arguments
/// * `dockerfile` - Path to the Dockerfile
///
/// # Errors
/// Returns an error if:
/// - Failed to open or read the Dockerfile
/// - Failed to parse the Dockerfile
/// - No base image found in Dockerfile
/// - Failed to execute podman pull command
pub fn pull_base_image(dockerfile: &Path) -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Use the error utils to handle file opening errors
    let file = std::fs::File::open(dockerfile).map_err(|e| {
        crate::utils::error_utils::into_boxed_error(
            e,
            &format!("Failed to open Dockerfile: {}", dockerfile.display()),
        )
    })?;

    let mut reader = BufReader::new(file);

    let mut content = String::new();
    reader.read_to_string(&mut content).map_err(|e| {
        crate::utils::error_utils::into_boxed_error(e, "Failed to read Dockerfile contents")
    })?;

    let dockerfile = Dockerfile::parse(&content).map_err(|e| {
        crate::utils::error_utils::into_boxed_error(e, "Failed to parse Dockerfile")
    })?;

    let from_img = dockerfile.instructions;
    let image_name = from_img
        .iter()
        .find_map(|instruction| {
            if let dockerfile_parser::Instruction::From(image, ..) = instruction {
                Some(image.image.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| crate::utils::error_utils::new_error("No base image found in Dockerfile"))?;

    // Use the command utilities
    // Convert SpannedString to regular String for the command args
    let image_name_str = image_name.to_string();
    crate::utils::cmd_utils::run_command_checked("podman", &["pull", &image_name_str])
}
