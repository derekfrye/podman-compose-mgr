mod image_details;
mod inference;

use std::path::PathBuf;
use std::sync::Arc;

use crate::domain::ScanResult;
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

    /// Scan for images using the discovery port.
    ///
    /// # Errors
    /// Returns an error if discovery fails.
    pub fn scan_images(
        &self,
        root: PathBuf,
        include: Vec<String>,
        exclude: Vec<String>,
    ) -> Result<ScanResult, PodmanComposeMgrError> {
        let opts = ScanOptions {
            root,
            include_patterns: include,
            exclude_patterns: exclude,
        };
        let discovery = self.discovery.scan(&opts)?;
        let local_images = self.podman.list_local_images().unwrap_or_default();
        let dockerfiles = Self::infer_dockerfiles(&discovery, &local_images);
        let makefiles = Self::infer_makefiles(&discovery, &local_images);
        Ok(ScanResult {
            images: discovery.images,
            dockerfiles,
            makefiles,
        })
    }
}
