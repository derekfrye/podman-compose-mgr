use crate::app::AppCore;
use crate::args::types::SimulateViewMode;
use crate::domain::{InferenceSource, ScanResult};
use crate::utils::log_utils::Logger;
use std::io;
use std::sync::Arc;

/// Run a dry-run style simulation of the TUI view and print rows to stdout.
pub fn simulate_view(
    args: &crate::args::Args,
    mode: SimulateViewMode,
    logger: &Logger,
) -> io::Result<()> {
    logger.info("Simulating TUI view (dry-run)");
    let discovery = Arc::new(crate::infra::discovery_adapter::FsDiscovery);
    let podman = Arc::new(crate::infra::podman_adapter::PodmanCli);
    let core = AppCore::new(discovery, podman);

    let scan = core
        .scan_images(
            args.path.clone(),
            args.include_path_patterns.clone(),
            args.exclude_path_patterns.clone(),
        )
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    match mode {
        SimulateViewMode::Dockerfile => print_dockerfiles(&scan),
        SimulateViewMode::Image => print_images(&scan),
        SimulateViewMode::Container => print_containers(&scan),
        SimulateViewMode::Folder => print_folders(&scan, &args.path),
    }

    Ok(())
}

fn print_dockerfiles(scan: &ScanResult) {
    for df in &scan.dockerfiles {
        let mut reason = match df.inference_source {
            InferenceSource::Quadlet => "quadlet neighbor".to_string(),
            InferenceSource::Compose => "compose neighbor".to_string(),
            InferenceSource::LocalhostRegistry => {
                if df.total_dockerfiles_in_dir > 1 {
                    "registry matched (more than one Dockerfile in the dir)".to_string()
                } else {
                    "registry matched".to_string()
                }
            }
            InferenceSource::Unknown => "no inference".to_string(),
        };
        if let Some(note) = &df.note {
            reason = note.clone();
        }

        let mut line = format!("[dry-run] {} -> {}", df.basename, reason);
        if let Some(img) = &df.inferred_image {
            line.push_str(&format!(" / registry name matched {img}"));
        }
        println!("{line}");
    }
}

fn print_images(scan: &ScanResult) {
    for img in &scan.images {
        let container = img.container.as_deref().unwrap_or("");
        println!(
            "[dry-run] image {} (container {:?}) from {}",
            img.image,
            container,
            img.source_dir.display()
        );
    }
}

fn print_containers(scan: &ScanResult) {
    print_images(scan);
}

fn print_folders(scan: &ScanResult, root: &std::path::Path) {
    let mut seen = std::collections::BTreeSet::new();
    for img in &scan.images {
        if let Ok(relative) = img.source_dir.strip_prefix(root) {
            if let Some(first) = relative.components().next() {
                seen.insert(first.as_os_str().to_string_lossy().to_string());
            }
        }
    }
    for dir in seen {
        println!("[dry-run] folder {dir}");
    }
}
