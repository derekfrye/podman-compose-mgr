use clap::Parser;
use std::path::PathBuf;

use super::validators::validate;
use crate::utils::path_utils::{check_readable_dir, check_writable_dir};

#[derive(Parser, Debug, Clone, serde::Serialize)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Search path for docker-compose files
    #[arg(
        short = 'p',
        long,
        value_name = "PATH",
        default_value = ".",
        value_parser = check_readable_dir
    )]
    pub path: PathBuf,

    /// Print extra stuff (use -v -v or --verbose --verbose for even more detail)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Regex pattern(s) to exclude paths, e.g., docker/archive or [^\.]+/archive
    #[arg(short, long)]
    pub exclude_path_patterns: Vec<String>,
    /// Regex pattern(s) to include paths. If both incl. and excl. are specified, excl. is applied first.
    #[arg(short, long)]
    pub include_path_patterns: Vec<String>,
    /// Build args to pass to podman build commands. Can be used multiple times.
    #[arg(short, long)]
    pub build_args: Vec<String>,

    /// Directory to use for temporary files
    #[arg(long, default_value = "/tmp", value_parser = check_writable_dir)]
    pub temp_file_path: PathBuf,

    /// Override the podman executable path
    #[arg(long, value_name = "PATH")]
    pub podman_bin: Option<PathBuf>,

    /// Use terminal UI mode
    #[arg(long)]
    pub tui: bool,

    /// Automatically start rebuild for all discovered images in TUI mode
    #[arg(long)]
    pub tui_rebuild_all: bool,
}

impl Default for Args {
    fn default() -> Self {
        // Use check_writable_dir to ensure the default path is valid or created
        // We need to handle the potential error here, perhaps by panicking
        // if the default /tmp isn't usable, as it's a fundamental requirement.
        let default_temp_path = check_writable_dir("/tmp")
            .expect("Default temporary directory '/tmp' must be writable or creatable.");

        Self {
            path: PathBuf::from("."),
            verbose: 0,
            exclude_path_patterns: Vec::new(),
            include_path_patterns: Vec::new(),
            build_args: Vec::new(),
            temp_file_path: default_temp_path,
            podman_bin: None,
            tui: false,
            tui_rebuild_all: false,
        }
    }
}

impl Args {
    /// Validate the secrets based on the mode, without modifying the Args
    ///
    /// # Errors
    ///
    /// Returns an error if the arguments are invalid for the selected mode.
    pub fn validate(&self) -> Result<(), String> {
        // Call the validate function from validators.rs
        validate(self)
    }
}
