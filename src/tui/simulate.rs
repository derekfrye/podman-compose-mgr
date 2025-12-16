use crate::app::AppCore;
use crate::args::types::SimulateViewMode;
use crate::domain::{InferenceSource, ScanResult};
use crate::ports::{DiscoveryPort, PodmanPort};
use crate::utils::log_utils::Logger;
use std::fmt::Write as FmtWrite;
use std::io::{self, Write};
use std::sync::Arc;

/// Run a dry-run style simulation of the TUI view and print rows to stdout.
///
/// # Errors
/// Returns an error if scanning images fails or writing output is not possible.
pub fn simulate_view(
    args: &crate::args::Args,
    mode: SimulateViewMode,
    logger: &Logger,
) -> io::Result<()> {
    logger.info("Simulating TUI view (dry-run)");
    let discovery = Arc::new(crate::infra::discovery_adapter::FsDiscovery);
    let podman: Arc<dyn PodmanPort> = if let Some(json_path) = &args.tui_simulate_podman_input_json
    {
        podman_from_json(json_path)?
    } else {
        Arc::new(crate::infra::podman_adapter::PodmanCli)
    };
    simulate_view_with_ports(args, mode, logger, discovery, podman, &mut io::stdout())
}

/// Test hook: run simulation with injected ports and custom writer.
///
/// # Errors
/// Returns an error if scanning images fails or writing output is not possible.
pub fn simulate_view_with_ports(
    args: &crate::args::Args,
    mode: SimulateViewMode,
    _logger: &Logger,
    discovery: Arc<dyn DiscoveryPort>,
    podman: Arc<dyn PodmanPort>,
    out: &mut dyn Write,
) -> io::Result<()> {
    let core = AppCore::new(discovery, podman);

    let scan = core
        .scan_images(
            args.path.clone(),
            args.include_path_patterns.clone(),
            args.exclude_path_patterns.clone(),
        )
        .map_err(|e| io::Error::other(e.to_string()))?;

    match mode {
        SimulateViewMode::Dockerfile => print_dockerfiles(&scan, out)?,
        SimulateViewMode::Image => print_images(&scan, out)?,
        SimulateViewMode::Container => print_containers(&scan, out)?,
        SimulateViewMode::Folder => print_folders(&scan, &args.path, out)?,
    }

    Ok(())
}

/// Build a podman port backed by local JSON.
///
/// # Errors
/// Returns an error if the JSON file cannot be read or parsed.
pub fn podman_from_json(path: &std::path::Path) -> io::Result<Arc<dyn PodmanPort>> {
    Ok(Arc::new(LocalJsonPodman::from_file(path)?))
}

struct LocalJsonPodman {
    images: Vec<crate::domain::LocalImageSummary>,
}

impl LocalJsonPodman {
    fn from_file(path: &std::path::Path) -> io::Result<Self> {
        let content = std::fs::read(path)?;
        let json =
            crate::infra::podman_adapter::parse_json_output(&content).map_err(io::Error::other)?;
        let images = crate::infra::podman_adapter::local_images_from_json(&json)
            .map_err(io::Error::other)?;
        Ok(Self { images })
    }
}

impl PodmanPort for LocalJsonPodman {
    fn image_created(
        &self,
        _image: &str,
    ) -> Result<chrono::DateTime<chrono::Local>, crate::errors::PodmanComposeMgrError> {
        Err(crate::errors::PodmanComposeMgrError::CommandExecution(
            Box::new(io::Error::other("not supported")),
        ))
    }

    fn image_modified(
        &self,
        _image: &str,
    ) -> Result<chrono::DateTime<chrono::Local>, crate::errors::PodmanComposeMgrError> {
        Err(crate::errors::PodmanComposeMgrError::CommandExecution(
            Box::new(io::Error::other("not supported")),
        ))
    }

    fn file_exists_and_readable(&self, file: &std::path::Path) -> bool {
        file.is_file()
    }

    fn list_local_images(
        &self,
    ) -> Result<Vec<crate::domain::LocalImageSummary>, crate::errors::PodmanComposeMgrError> {
        Ok(self.images.clone())
    }
}

fn print_dockerfiles(scan: &ScanResult, out: &mut dyn Write) -> io::Result<()> {
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
            reason.clone_from(note);
        }

        let mut line = format!("[dry-run] {} -> {}", df.basename, reason);
        if let Some(img) = &df.inferred_image {
            write!(&mut line, " / registry name matched {img}").map_err(io::Error::other)?;
        }
        writeln!(out, "{line}")?;
    }
    Ok(())
}

fn print_images(scan: &ScanResult, out: &mut dyn Write) -> io::Result<()> {
    for img in &scan.images {
        let container = img.container.as_deref().unwrap_or("");
        writeln!(
            out,
            "[dry-run] image {} (container {:?}) from {}",
            img.image,
            container,
            img.source_dir.display()
        )?;
    }
    Ok(())
}

fn print_containers(scan: &ScanResult, out: &mut dyn Write) -> io::Result<()> {
    print_images(scan, out)
}

fn print_folders(scan: &ScanResult, root: &std::path::Path, out: &mut dyn Write) -> io::Result<()> {
    let mut seen = std::collections::BTreeSet::new();
    for img in &scan.images {
        if let Ok(relative) = img.source_dir.strip_prefix(root)
            && let Some(first) = relative.components().next()
        {
            seen.insert(first.as_os_str().to_string_lossy().to_string());
        }
    }
    for dir in seen {
        writeln!(out, "[dry-run] folder {dir}")?;
    }
    Ok(())
}
