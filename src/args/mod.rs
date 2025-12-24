// Public modules
pub mod types;
mod validators;

// Re-export everything from the submodules
pub use types::*;
pub use validators::*;

use clap::{CommandFactory, FromArgMatches};
use clap::parser::ValueSource;
use std::fs;
use std::path::Path;
use std::process;

use crate::utils::path_utils::{check_readable_dir_path, check_writable_dir};

#[derive(Debug, Default, serde::Deserialize)]
struct ConfigFileArgs {
    #[serde(alias = "path")]
    path: Option<std::path::PathBuf>,
    verbose: Option<u8>,
    #[serde(alias = "exclude-path-patterns")]
    exclude_path_patterns: Option<Vec<String>>,
    #[serde(alias = "include-path-patterns")]
    include_path_patterns: Option<Vec<String>>,
    #[serde(alias = "build-args")]
    build_args: Option<Vec<String>>,
    #[serde(alias = "temp-file-path")]
    temp_file_path: Option<std::path::PathBuf>,
    #[serde(alias = "podman-bin")]
    podman_bin: Option<std::path::PathBuf>,
    #[serde(alias = "no-cache")]
    no_cache: Option<bool>,
    #[serde(alias = "one-shot")]
    one_shot: Option<bool>,
    #[serde(alias = "dry-run")]
    dry_run: Option<bool>,
    tui: Option<bool>,
    #[serde(alias = "tui-rebuild-all")]
    tui_rebuild_all: Option<bool>,
    #[serde(alias = "rebuild-view-line-buffer-max")]
    rebuild_view_line_buffer_max: Option<usize>,
    #[serde(alias = "tui-simulate")]
    tui_simulate: Option<SimulateViewMode>,
    #[serde(alias = "tui-simulate-podman-input-json")]
    tui_simulate_podman_input_json: Option<std::path::PathBuf>,
}

fn read_config_toml(path: &Path) -> Result<ConfigFileArgs, String> {
    let contents = fs::read_to_string(path)
        .map_err(|e| format!("Unable to read config TOML '{}': {e}", path.display()))?;
    toml::from_str(&contents)
        .map_err(|e| format!("Invalid TOML in '{}': {e}", path.display()))
}

fn is_cli_set(matches: &clap::ArgMatches, id: &str) -> bool {
    matches
        .value_source(id)
        .is_some_and(|source| matches!(source, ValueSource::CommandLine))
}

fn apply_config_toml(args: &mut Args, matches: &clap::ArgMatches) -> Result<(), String> {
    let Some(config_path) = args.config_toml.as_ref() else {
        return Ok(());
    };
    let config = read_config_toml(config_path)?;

    if !is_cli_set(matches, "path") {
        if let Some(path) = config.path {
            args.path = check_readable_dir_path(&path)?;
        }
    }
    if !is_cli_set(matches, "verbose") {
        if let Some(verbose) = config.verbose {
            args.verbose = verbose;
        }
    }
    if !is_cli_set(matches, "exclude-path-patterns") {
        if let Some(patterns) = config.exclude_path_patterns {
            args.exclude_path_patterns = patterns;
        }
    }
    if !is_cli_set(matches, "include-path-patterns") {
        if let Some(patterns) = config.include_path_patterns {
            args.include_path_patterns = patterns;
        }
    }
    if !is_cli_set(matches, "build-args") {
        if let Some(build_args) = config.build_args {
            args.build_args = build_args;
        }
    }
    if !is_cli_set(matches, "temp-file-path") {
        if let Some(temp_path) = config.temp_file_path {
            let temp_str = temp_path
                .to_str()
                .ok_or_else(|| "Invalid temp path: contains non-UTF-8 characters".to_string())?;
            args.temp_file_path = check_writable_dir(temp_str)?;
        }
    }
    if !is_cli_set(matches, "podman-bin") {
        if let Some(podman_bin) = config.podman_bin {
            args.podman_bin = Some(podman_bin);
        }
    }
    if !is_cli_set(matches, "no-cache") {
        if let Some(no_cache) = config.no_cache {
            args.no_cache = no_cache;
        }
    }
    if !is_cli_set(matches, "one-shot") {
        if let Some(one_shot) = config.one_shot {
            args.one_shot.one_shot = one_shot;
        }
    }
    if !is_cli_set(matches, "dry-run") {
        if let Some(dry_run) = config.dry_run {
            args.one_shot.dry_run = dry_run;
        }
    }
    if !is_cli_set(matches, "tui") {
        if let Some(tui) = config.tui {
            args.tui.enabled = tui;
        }
    }
    if !is_cli_set(matches, "tui-rebuild-all") {
        if let Some(rebuild_all) = config.tui_rebuild_all {
            args.tui.rebuild_all = rebuild_all;
        }
    }
    if !is_cli_set(matches, "rebuild-view-line-buffer-max") {
        if let Some(max) = config.rebuild_view_line_buffer_max {
            args.rebuild_view_line_buffer_max = max;
        }
    }
    if !is_cli_set(matches, "tui-simulate") {
        if let Some(sim_mode) = config.tui_simulate {
            args.tui_simulate = Some(sim_mode);
        }
    }
    if !is_cli_set(matches, "tui-simulate-podman-input-json") {
        if let Some(path) = config.tui_simulate_podman_input_json {
            args.tui_simulate_podman_input_json = Some(path);
        }
    }

    Ok(())
}

/// Parse command line arguments and perform validation with processing
///
/// This function:
/// 1. Parses command line arguments
/// 2. For `SecretInitialize` mode, processes the init filepath if needed
/// 3. Returns the validated Args structure
///
/// # Returns
///
/// * `Args` - The validated arguments
///
/// # Panics
///
/// Panics if validation fails
#[must_use]
pub fn args_checks() -> Args {
    let matches = Args::command().get_matches();
    let mut args = Args::from_arg_matches(&matches).unwrap_or_else(|err| err.exit());

    if let Err(e) = apply_config_toml(&mut args, &matches) {
        eprintln!("Error: {e}");
        process::exit(1);
    }

    // Validate the arguments
    if let Err(e) = args.validate() {
        eprintln!("Error: {e}");
        process::exit(1);
    }

    args
}
