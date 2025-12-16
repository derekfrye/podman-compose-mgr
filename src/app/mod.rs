use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::domain::{
    DiscoveryResult, DockerfileInference, ImageDetails, InferenceSource, LocalImageSummary,
    ScanResult,
};
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
        let dockerfiles = self.infer_dockerfiles(discovery, &local_images);
        Ok(dockerfiles)
    }

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

        // Format relative time strings in app layer, keep UI simpler
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
                && let Some(name) = candidate.file_name() {
                    return Some(name.to_string_lossy().into_owned());
                }
        }
        None
    }

    fn dockerfile_candidates(source_dir: &Path, entry_path: Option<&Path>) -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        if let Some(entry) = entry_path {
            if entry.extension().and_then(|ext| ext.to_str()) == Some("container")
                && let (Some(parent), Some(stem)) = (entry.parent(), entry.file_stem()) {
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

    fn infer_dockerfiles(
        &self,
        discovery: DiscoveryResult,
        local_images: &[LocalImageSummary],
    ) -> ScanResult {
        let mut inferred = Vec::new();
        for dockerfile in discovery.dockerfiles {
            let inference_source;
            let inferred_image;
            let created_time_ago;
            let note;

            if let Some((source, image)) = dockerfile.neighbor_image.clone() {
                inference_source = source;
                inferred_image = Some(image.clone());
                created_time_ago = self.find_created_for(&image, local_images);
                note = Some("single neighbor file".to_string());
            } else {
                let suffix = dockerfile
                    .basename
                    .strip_prefix("Dockerfile")
                    .unwrap_or(&dockerfile.basename);
                let suffix = suffix.trim_start_matches('.');
                let match_entry = if suffix.is_empty() {
                    None
                } else {
                    match_localhost_image(suffix, local_images)
                };
                if let Some(entry) = match_entry {
                    inference_source = InferenceSource::LocalhostRegistry;
                    inferred_image = Some(format!("{}:{}", entry.repository, entry.tag));
                    created_time_ago = entry
                        .created
                        .map(crate::utils::podman_utils::format_time_ago);
                    if dockerfile.total_dockerfiles_in_dir > 1 {
                        note = Some(
                            "registry matched (more than one Dockerfile in the dir)".to_string(),
                        );
                    } else {
                        note = Some("registry matched".to_string());
                    }
                } else {
                    inference_source = InferenceSource::Unknown;
                    inferred_image = None;
                    created_time_ago = None;
                    note = None;
                }
            }

            inferred.push(DockerfileInference {
                dockerfile_path: dockerfile.dockerfile_path,
                source_dir: dockerfile.source_dir,
                basename: dockerfile.basename,
                inferred_image,
                inference_source,
                created_time_ago,
                total_dockerfiles_in_dir: dockerfile.total_dockerfiles_in_dir,
                neighbor_file_count: dockerfile.neighbor_file_count,
                note,
            });
        }

        ScanResult {
            images: discovery.images,
            dockerfiles: inferred,
        }
    }

    fn find_created_for(&self, image: &str, local_images: &[LocalImageSummary]) -> Option<String> {
        match_localhost_image_exact(image, local_images).and_then(|entry| {
            entry
                .created
                .map(crate::utils::podman_utils::format_time_ago)
        })
    }
}

fn match_localhost_image<'a>(
    suffix: &str,
    local_images: &'a [LocalImageSummary],
) -> Option<&'a LocalImageSummary> {
    let mut candidates: Vec<&LocalImageSummary> = local_images
        .iter()
        .filter(|img| {
            img.repository.starts_with("localhost")
                && (img.repository.ends_with(&format!("/{suffix}"))
                    || img.repository.split('/').next_back() == Some(suffix))
        })
        .collect();
    candidates.sort_by(|a, b| b.created.cmp(&a.created));
    candidates.into_iter().next()
}

fn match_localhost_image_exact<'a>(
    name: &str,
    local_images: &'a [LocalImageSummary],
) -> Option<&'a LocalImageSummary> {
    local_images
        .iter()
        .find(|img| format!("{}:{}", img.repository, img.tag) == name)
}
