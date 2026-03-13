use super::AppCore;
use crate::domain::ImageDetails;
use crate::errors::PodmanComposeMgrError;
use std::path::{Path, PathBuf};

impl AppCore {
    /// Get image details for display.
    ///
    /// # Errors
    /// Returns an error if filesystem checks fail unexpectedly.
    pub fn image_details(
        &self,
        image: &str,
        source_dir: &Path,
        entry_path: Option<&Path>,
    ) -> Result<ImageDetails, PodmanComposeMgrError> {
        let created = self.podman.image_created(image).ok();
        let pulled = self.podman.image_modified(image).ok();

        let created_time_ago = created.map(crate::utils::podman_utils::datetime::format_time_ago);
        let pulled_time_ago = pulled.map(crate::utils::podman_utils::datetime::format_time_ago);

        let dockerfile_name = self.locate_dockerfile(source_dir, entry_path);
        let makefile = source_dir.join("Makefile");
        let has_makefile = self.podman.file_exists_and_readable(&makefile);

        Ok(ImageDetails {
            created_time_ago,
            pulled_time_ago,
            dockerfile_name,
            has_makefile,
        })
    }

    fn locate_dockerfile(&self, source_dir: &Path, entry_path: Option<&Path>) -> Option<String> {
        for candidate in Self::dockerfile_candidates(source_dir, entry_path) {
            if self.podman.file_exists_and_readable(&candidate)
                && let Some(name) = candidate.file_name()
            {
                return Some(name.to_string_lossy().into_owned());
            }
        }
        None
    }

    fn dockerfile_candidates(source_dir: &Path, entry_path: Option<&Path>) -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        if let Some(entry) = entry_path {
            if entry.extension().and_then(|ext| ext.to_str()) == Some("container")
                && let (Some(parent), Some(stem)) = (entry.parent(), entry.file_stem())
            {
                let suffix = stem.to_string_lossy();
                candidates.push(parent.join(format!("Dockerfile.{suffix}")));
            }
            if let Some(parent) = entry.parent() {
                candidates.push(parent.join("Dockerfile"));
            }
        }

        let fallback = source_dir.join("Dockerfile");
        if candidates.iter().all(|cand| cand != &fallback) {
            candidates.push(fallback);
        }

        candidates
    }
}
