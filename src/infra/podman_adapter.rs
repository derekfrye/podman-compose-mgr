use crate::errors::PodmanComposeMgrError;
use crate::ports::PodmanPort;
use chrono::{DateTime, Local};
use std::path::Path;

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
}
