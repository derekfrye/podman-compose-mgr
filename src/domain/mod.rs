use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DiscoveredImage {
    pub image: String,
    pub container: Option<String>,
    pub source_dir: PathBuf,
    pub entry_path: PathBuf,
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
