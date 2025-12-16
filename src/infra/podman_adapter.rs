use crate::domain::LocalImageSummary;
use crate::errors::PodmanComposeMgrError;
use crate::ports::PodmanPort;
use chrono::{DateTime, Local};
use std::path::Path;
use std::process::Command;

pub struct PodmanCli;

impl PodmanPort for PodmanCli {
    fn image_created(&self, image: &str) -> Result<DateTime<Local>, PodmanComposeMgrError> {
        crate::utils::podman_utils::image::get_podman_image_upstream_create_time(image)
            .map_err(|e| PodmanComposeMgrError::CommandExecution(Box::new(e)))
    }

    fn image_modified(&self, image: &str) -> Result<DateTime<Local>, PodmanComposeMgrError> {
        crate::utils::podman_utils::image::get_podman_ondisk_modify_time(image)
            .map_err(|e| PodmanComposeMgrError::CommandExecution(Box::new(e)))
    }

    fn file_exists_and_readable(&self, file: &Path) -> bool {
        crate::utils::podman_utils::terminal::file_exists_and_readable(file)
    }

    fn list_local_images(&self) -> Result<Vec<LocalImageSummary>, PodmanComposeMgrError> {
        let output = Command::new(crate::utils::podman_utils::resolve_podman_binary())
            .args(["image", "ls", "--format", "json"])
            .output()
            .map_err(|e| PodmanComposeMgrError::CommandExecution(Box::new(e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(PodmanComposeMgrError::CommandExecution(Box::new(
                std::io::Error::new(std::io::ErrorKind::Other, stderr),
            )));
        }

        let entries: Vec<PodmanImageEntry> = serde_json::from_slice(&output.stdout)
            .map_err(|e| PodmanComposeMgrError::CommandExecution(Box::new(e)))?;

        let mapped = entries
            .into_iter()
            .map(|img| LocalImageSummary {
                repository: img.repository.unwrap_or_default(),
                tag: img.tag.unwrap_or_default(),
                created: img.created_at.as_deref().and_then(|s| {
                    crate::utils::podman_utils::datetime::convert_str_to_date(s).ok()
                }),
            })
            .collect();
        Ok(mapped)
    }
}

#[derive(serde::Deserialize)]
struct PodmanImageEntry {
    #[serde(rename = "Repository")]
    repository: Option<String>,
    #[serde(rename = "Tag")]
    tag: Option<String>,
    #[serde(rename = "CreatedAt")]
    created_at: Option<String>,
}
