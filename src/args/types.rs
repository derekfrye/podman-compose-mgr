use clap::Parser;
use std::path::PathBuf;

use super::validators::validate;
use crate::utils::path_utils::{check_readable_dir, check_writable_dir};

pub const REBUILD_VIEW_LINE_BUFFER_DEFAULT: usize = 4_096;

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

    /// Disable podman build cache
    #[arg(long)]
    pub no_cache: bool,

    /// Run discovery once and automatically build or pull each image
    #[command(flatten)]
    pub one_shot: OneShotArgs,

    /// Use terminal UI mode
    #[command(flatten)]
    pub tui: TuiArgs,

    /// Maximum number of lines retained in the rebuild TUI output
    #[arg(
        long = "rebuild-view-line-buffer-max",
        value_name = "LINES",
        default_value_t = REBUILD_VIEW_LINE_BUFFER_DEFAULT,
        value_parser = clap::value_parser!(usize)
    )]
    pub rebuild_view_line_buffer_max: usize,

    /// Simulate a TUI view in dry-run mode (e.g., view-mode-dockerfile)
    #[arg(long = "tui-simulate", value_parser = parse_sim_view)]
    pub tui_simulate: Option<SimulateViewMode>,

    /// JSON input file to use for podman image listing during TUI simulation
    #[arg(long = "tui-simulate-podman-input-json")]
    pub tui_simulate_podman_input_json: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, clap::Args, Default)]
pub struct OneShotArgs {
    /// Run discovery once and automatically build or pull each image
    #[arg(long = "one-shot")]
    pub one_shot: bool,

    /// Print which images would be built or pulled when using --one-shot
    #[arg(long, requires = "one_shot")]
    pub dry_run: bool,
}

impl OneShotArgs {
    #[must_use]
    pub fn enabled(self) -> bool {
        self.one_shot
    }

    #[must_use]
    pub fn is_dry_run(self) -> bool {
        self.one_shot && self.dry_run
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, clap::Args, Default)]
pub struct TuiArgs {
    /// Use terminal UI mode
    #[arg(long = "tui")]
    pub enabled: bool,

    /// Automatically start rebuild for all discovered images in TUI mode
    #[arg(long = "tui-rebuild-all", requires = "enabled")]
    pub rebuild_all: bool,
}

impl TuiArgs {
    #[must_use]
    pub fn enabled(self) -> bool {
        self.enabled
    }

    #[must_use]
    pub fn rebuild_all(self) -> bool {
        self.enabled && self.rebuild_all
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum SimulateViewMode {
    Container,
    Image,
    Folder,
    Dockerfile,
}

fn parse_sim_view(raw: &str) -> Result<SimulateViewMode, String> {
    match raw {
        "view-mode-container" | "container" => Ok(SimulateViewMode::Container),
        "view-mode-image" | "image" => Ok(SimulateViewMode::Image),
        "view-mode-folder" | "folder" => Ok(SimulateViewMode::Folder),
        "view-mode-dockerfile" | "dockerfile" => Ok(SimulateViewMode::Dockerfile),
        other => Err(format!(
            "invalid tui-simulate value '{other}', expected one of: view-mode-container, view-mode-image, view-mode-folder, view-mode-dockerfile"
        )),
    }
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
            no_cache: false,
            one_shot: OneShotArgs::default(),
            tui: TuiArgs::default(),
            rebuild_view_line_buffer_max: REBUILD_VIEW_LINE_BUFFER_DEFAULT,
            tui_simulate: None,
            tui_simulate_podman_input_json: None,
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
