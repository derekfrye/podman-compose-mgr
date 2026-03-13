use super::expansion_details::compute_details_for;
use crate::app::AppCore;
use crate::domain::{DiscoveryResult, InferenceSource};
use crate::errors::PodmanComposeMgrError;
use crate::ports::{DiscoveryPort, PodmanPort, ScanOptions};
use crate::tui::app::ViewMode;
use crate::tui::app::state::MakefileRowExtra;
use chrono::{Duration, Local};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct FakeDiscovery;

impl DiscoveryPort for FakeDiscovery {
    fn scan(&self, _opts: &ScanOptions) -> Result<DiscoveryResult, PodmanComposeMgrError> {
        Ok(DiscoveryResult {
            images: Vec::new(),
            dockerfiles: Vec::new(),
            makefiles: Vec::new(),
        })
    }
}

struct FakePodman {
    created_at: HashMap<String, chrono::DateTime<Local>>,
}

impl PodmanPort for FakePodman {
    fn image_created(&self, image: &str) -> Result<chrono::DateTime<Local>, PodmanComposeMgrError> {
        self.created_at.get(image).copied().ok_or_else(|| {
            PodmanComposeMgrError::CommandExecution(Box::new(std::io::Error::other(
                "image not found",
            )))
        })
    }

    fn image_modified(
        &self,
        _image: &str,
    ) -> Result<chrono::DateTime<Local>, PodmanComposeMgrError> {
        Err(PodmanComposeMgrError::CommandExecution(Box::new(
            std::io::Error::other("not needed"),
        )))
    }

    fn file_exists_and_readable(&self, _file: &Path) -> bool {
        false
    }

    fn list_local_images(
        &self,
    ) -> Result<Vec<crate::domain::LocalImageSummary>, PodmanComposeMgrError> {
        Ok(Vec::new())
    }
}

#[test]
fn makefile_details_fall_back_to_live_image_created_time() {
    let image = "localhost/djf/m-golf-srvless:latest";
    let created_at = Local::now() - Duration::hours(3);
    let expected_created = crate::utils::podman_utils::datetime::format_time_ago(created_at);
    let core = AppCore::new(
        Arc::new(FakeDiscovery),
        Arc::new(FakePodman {
            created_at: HashMap::from([(image.to_string(), created_at)]),
        }),
    );
    let source_dir = PathBuf::from("tests/test1");
    let entry_path = source_dir.join("Makefile");

    let details = compute_details_for(
        &core,
        image,
        &source_dir,
        Some(&entry_path),
        ViewMode::ByMakefile,
        None,
        Some(&MakefileRowExtra {
            source: InferenceSource::Quadlet,
            makefile_name: "Makefile".to_string(),
            make_target: Some("clean".to_string()),
            quadlet_basename: Some("m-miniflare.container".to_string()),
            image_name: Some(image.to_string()),
            image_names: vec![image.to_string()],
            created_time_ago: None,
            note: Some("single neighbor file".to_string()),
        }),
    );

    assert!(details.contains(&format!("Created: {expected_created}")));
}

#[test]
fn makefile_details_show_multiple_recipe_images() {
    let first = "djf/m-ffmpeg_base";
    let second = "djf/m-ffmpeg";
    let first_created_at = Local::now() - Duration::hours(5);
    let second_created_at = Local::now() - Duration::hours(2);
    let first_created = crate::utils::podman_utils::datetime::format_time_ago(first_created_at);
    let second_created = crate::utils::podman_utils::datetime::format_time_ago(second_created_at);
    let core = AppCore::new(
        Arc::new(FakeDiscovery),
        Arc::new(FakePodman {
            created_at: HashMap::from([
                (first.to_string(), first_created_at),
                (second.to_string(), second_created_at),
            ]),
        }),
    );
    let source_dir = PathBuf::from("tests/test1");
    let entry_path = source_dir.join("Makefile");

    let details = compute_details_for(
        &core,
        first,
        &source_dir,
        Some(&entry_path),
        ViewMode::ByMakefile,
        None,
        Some(&MakefileRowExtra {
            source: InferenceSource::Unknown,
            makefile_name: "golf: ffmpeg".to_string(),
            make_target: Some("ffmpeg".to_string()),
            quadlet_basename: None,
            image_name: Some(first.to_string()),
            image_names: vec![first.to_string(), second.to_string()],
            created_time_ago: None,
            note: None,
        }),
    );

    assert!(details.contains(&format!("Image: {first}")));
    assert!(details.contains(&format!("Created: {first_created}")));
    assert!(details.contains(&format!("Image: {second}")));
    assert!(details.contains(&format!("Created: {second_created}")));
}
