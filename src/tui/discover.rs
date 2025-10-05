use crate::Args;
use crate::app::AppCore;
use crate::infra::{discovery_adapter::FsDiscovery, podman_adapter::PodmanCli};
use crate::utils::log_utils::Logger;

// Re-export type at this path for compatibility
pub use crate::domain::DiscoveredImage;

// Compatibility shim: old free function that performs a scan.
// Internally wires default adapters and uses the app core.
pub fn scan_images(args: &Args, _logger: &Logger) -> Vec<DiscoveredImage> {
    let discovery = std::sync::Arc::new(FsDiscovery);
    let podman = std::sync::Arc::new(PodmanCli);
    let core = AppCore::new(discovery, podman);
    core.scan_images(
        args.path.clone(),
        args.include_path_patterns.clone(),
        args.exclude_path_patterns.clone(),
    )
    .unwrap_or_default()
}
