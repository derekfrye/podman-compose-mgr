use chrono::{DateTime, Local};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DiscoveredImage {
    pub image: String,
    pub container: Option<String>,
    pub source_dir: PathBuf,
    pub entry_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InferenceSource {
    Quadlet,
    Compose,
    LocalhostRegistry,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredDockerfile {
    pub dockerfile_path: PathBuf,
    pub source_dir: PathBuf,
    pub basename: String,
    pub quadlet_basename: Option<String>,
    pub neighbor_image: Option<(InferenceSource, String)>,
    pub total_dockerfiles_in_dir: usize,
    pub neighbor_file_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DockerfileInference {
    pub dockerfile_path: PathBuf,
    pub source_dir: PathBuf,
    pub basename: String,
    pub quadlet_basename: Option<String>,
    pub inferred_image: Option<String>,
    pub inference_source: InferenceSource,
    pub created_time_ago: Option<String>,
    pub total_dockerfiles_in_dir: usize,
    pub neighbor_file_count: usize,
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveryResult {
    pub images: Vec<DiscoveredImage>,
    pub dockerfiles: Vec<DiscoveredDockerfile>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ScanResult {
    pub images: Vec<DiscoveredImage>,
    pub dockerfiles: Vec<DockerfileInference>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalImageSummary {
    pub repository: String,
    pub tag: String,
    pub created: Option<DateTime<Local>>,
}

// Data returned when asking for details about an image.
// The TUI formats these values for display.
#[derive(Clone, Debug)]
pub struct ImageDetails {
    pub created_time_ago: Option<String>,
    pub pulled_time_ago: Option<String>,
    pub dockerfile_name: Option<String>,
    pub has_makefile: bool,
}
