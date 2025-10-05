use crate::domain::DiscoveredImage;
use crate::errors::PodmanComposeMgrError;
use chrono::Local;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

pub trait PodmanPort: Send + Sync {
    /// Return upstream creation time for an image.
    ///
    /// # Errors
    /// Returns an error if the underlying command fails or the output cannot be parsed.
    fn image_created(&self, image: &str) -> Result<chrono::DateTime<Local>, PodmanComposeMgrError>;
    /// Return local on-disk modification time for an image.
    ///
    /// # Errors
    /// Returns an error if the underlying command fails or the output cannot be parsed.
    fn image_modified(&self, image: &str)
    -> Result<chrono::DateTime<Local>, PodmanComposeMgrError>;
    fn file_exists_and_readable(&self, file: &Path) -> bool;
}

pub struct ScanOptions {
    pub root: PathBuf,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

pub trait DiscoveryPort: Send + Sync {
    /// Scan the filesystem for docker compose/container metadata.
    ///
    /// # Errors
    /// Returns an error if pattern compilation or file IO fails.
    fn scan(&self, opts: &ScanOptions) -> Result<Vec<DiscoveredImage>, PodmanComposeMgrError>;
}

// Interrupt port for graceful shutdown without OS signals in tests.
pub trait InterruptPort: Send {
    // Returns a one-shot receiver that yields on interrupt. May only be called once.
    fn subscribe(self: Box<Self>) -> Receiver<()>;
}
