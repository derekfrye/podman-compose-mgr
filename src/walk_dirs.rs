use crate::errors::PodmanComposeMgrError;
use regex::Regex;
use walkdir::WalkDir;

use crate::{
    Args,
    image_build::{self as build},
    interfaces::{
        CommandHelper, DefaultCommandHelper, DefaultReadInteractiveInputHelper,
        ReadInteractiveInputHelper,
    },
    tui,
    utils::log_utils::Logger,
};
use std::sync::mpsc::Receiver;

/// Main function that uses the default helper implementations
///
/// This function handles errors internally and prints them rather than propagating them
pub fn walk_dirs(args: &Args, logger: &Logger, tui_mode: bool) {
    // Use default implementations
    let cmd_helper = DefaultCommandHelper;
    let read_val_helper = DefaultReadInteractiveInputHelper;

    // Call the injectable version with default implementations
    if let Err(e) =
        walk_dirs_with_helpers_and_interrupt(args, &cmd_helper, &read_val_helper, logger, None)
    {
        eprintln!("Error processing directories: {e}");
    }

    // If TUI mode is enabled, launch the TUI
    if tui_mode {
        logger.info("Starting TUI mode...");
        if let Err(e) = tui::run(args, logger) {
            eprintln!("Error starting TUI: {e}");
        }
    }
}

/// Version of `walk_dirs` that accepts dependency injection for testing
///
/// # Arguments
///
/// * `args` - Command line arguments
/// * `cmd_helper` - Command helper implementation
/// * `read_val_helper` - Interactive input helper implementation
/// * `logger` - Logger instance
///
/// # Returns
///
/// * `Result<(), PodmanComposeMgrError>` - Success or error
///
/// # Errors
///
/// Returns an error if directory walking fails or rebuild operations encounter errors.
pub fn walk_dirs_with_helpers<C: CommandHelper, R: ReadInteractiveInputHelper>(
    args: &Args,
    cmd_helper: &C,
    read_val_helper: &R,
    logger: &Logger,
) -> Result<(), PodmanComposeMgrError> {
    walk_dirs_with_helpers_and_interrupt(args, cmd_helper, read_val_helper, logger, None)
}

/// Injectable version with optional interrupt receiver for graceful cancellation
/// Walk directories and handle rebuilds, with optional interrupt support.
///
/// # Errors
/// Returns an error if regex compilation fails or rebuild operations return an error.
pub fn walk_dirs_with_helpers_and_interrupt<C: CommandHelper, R: ReadInteractiveInputHelper>(
    args: &Args,
    cmd_helper: &C,
    read_val_helper: &R,
    logger: &Logger,
    interrupt: Option<&Receiver<()>>,
) -> Result<(), PodmanComposeMgrError> {
    let exclude_patterns =
        compile_patterns(logger, "Excluding paths", &args.exclude_path_patterns)?;
    let include_patterns =
        compile_patterns(logger, "Including paths", &args.include_path_patterns)?;

    logger.info(&format!("Rebuild images in path: {}", args.path.display()));

    let mut manager = Some(build::rebuild::RebuildManager::new(
        cmd_helper,
        read_val_helper,
    ));

    for entry in walk_target_entries(&args.path) {
        if interrupt_triggered(interrupt, logger) {
            break;
        }

        if !is_target_entry(&entry) {
            continue;
        }

        let Some(entry_path) = entry.path().to_str() else {
            log_invalid_path(&entry);
            continue;
        };

        if should_skip_entry(entry_path, &exclude_patterns, &include_patterns, logger) {
            continue;
        }

        if let Some(ref mut mgr) = manager
            && let Err(e) = mgr.rebuild(&entry, args)
        {
            eprintln!("Error rebuilding from {entry_path}: {e}");
        }
    }

    Ok(())
}

fn compile_patterns(
    logger: &Logger,
    label: &str,
    patterns: &[String],
) -> Result<Vec<Regex>, PodmanComposeMgrError> {
    if patterns.is_empty() {
        return Ok(Vec::new());
    }

    logger.info(&format!("{label}: {patterns:?}"));
    patterns
        .iter()
        .map(|pattern| Regex::new(pattern))
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn walk_target_entries(path: &std::path::Path) -> impl Iterator<Item = walkdir::DirEntry> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(std::result::Result::ok)
}

fn interrupt_triggered(interrupt: Option<&Receiver<()>>, logger: &Logger) -> bool {
    let Some(rx) = interrupt else {
        return false;
    };

    match rx.try_recv() {
        Ok(()) => {
            logger.info("Interrupt received; stopping traversal.");
            true
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => false,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => false,
    }
}

fn is_target_entry(entry: &walkdir::DirEntry) -> bool {
    let is_file = entry.file_type().is_file();
    if !is_file {
        return false;
    }

    entry.file_name() == "docker-compose.yml"
        || entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.ends_with(".container"))
}

fn log_invalid_path(entry: &walkdir::DirEntry) {
    eprintln!(
        "Skipping path with invalid UTF-8: {}",
        entry.path().display()
    );
}

fn should_skip_entry(
    entry_path: &str,
    exclude_patterns: &[Regex],
    include_patterns: &[Regex],
    logger: &Logger,
) -> bool {
    if !exclude_patterns.is_empty()
        && exclude_patterns
            .iter()
            .any(|pattern| pattern.is_match(entry_path))
    {
        return true;
    }

    if !include_patterns.is_empty()
        && include_patterns
            .iter()
            .all(|pattern| !pattern.is_match(entry_path))
    {
        logger.debug(&format!(
            "Skipping path as it doesn't match any include pattern: {entry_path}"
        ));
        return true;
    }

    false
}
