use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::domain::{DiscoveredImage, ImageDetails};
use crate::errors::PodmanComposeMgrError;
use crate::ports::{DiscoveryPort, PodmanPort, ScanOptions};

pub struct AppCore {
    discovery: Arc<dyn DiscoveryPort>,
    podman: Arc<dyn PodmanPort>,
}

impl AppCore {
    pub fn new(discovery: Arc<dyn DiscoveryPort>, podman: Arc<dyn PodmanPort>) -> Self {
        Self { discovery, podman }
    }

    pub fn scan_images(
        &self,
        root: PathBuf,
        include: Vec<String>,
        exclude: Vec<String>,
    ) -> Result<Vec<DiscoveredImage>, PodmanComposeMgrError> {
        let opts = ScanOptions { root, include_patterns: include, exclude_patterns: exclude };
        self.discovery.scan(&opts)
    }

    pub fn image_details(&self, image: &str, source_dir: &Path) -> Result<ImageDetails, PodmanComposeMgrError> {
        let created = self.podman.image_created(image).ok();
        let pulled = self.podman.image_modified(image).ok();

        // Format relative time strings in app layer, keep UI simpler
        let created_time_ago = created.map(crate::utils::podman_utils::datetime::format_time_ago);
        let pulled_time_ago = pulled.map(crate::utils::podman_utils::datetime::format_time_ago);

        let dockerfile = source_dir.join("Dockerfile");
        let makefile = source_dir.join("Makefile");
        let has_dockerfile = self.podman.file_exists_and_readable(&dockerfile);
        let has_makefile = self.podman.file_exists_and_readable(&makefile);

        Ok(ImageDetails { created_time_ago, pulled_time_ago, has_dockerfile, has_makefile })
    }
}
