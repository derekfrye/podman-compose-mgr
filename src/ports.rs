use crate::domain::DiscoveredImage;
use crate::errors::PodmanComposeMgrError;
use chrono::Local;
use std::path::{Path, PathBuf};

pub trait PodmanPort: Send + Sync {
    fn image_created(&self, image: &str) -> Result<chrono::DateTime<Local>, PodmanComposeMgrError>;
    fn image_modified(&self, image: &str) -> Result<chrono::DateTime<Local>, PodmanComposeMgrError>;
    fn file_exists_and_readable(&self, file: &Path) -> bool;
}

pub struct ScanOptions {
    pub root: PathBuf,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

pub trait DiscoveryPort: Send + Sync {
    fn scan(&self, opts: &ScanOptions) -> Result<Vec<DiscoveredImage>, PodmanComposeMgrError>;
}

