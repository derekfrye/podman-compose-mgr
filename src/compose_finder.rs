use std::mem;

use regex::Regex;
use thiserror::Error;
use walkdir::WalkDir;

use crate::{
    Args, args,
    build::{self, rebuild::RebuildManager},
    interfaces::{CommandHelper, DefaultCommandHelper, DefaultReadValHelper, ReadValHelper},
    restartsvcs,
};

#[derive(Debug, Error)]
pub enum StartError {
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    
    #[error("Path contains invalid UTF-8: {0}")]
    InvalidPath(String),
    
    #[error("Rebuild error: {0}")]
    RebuildError(String),
}

/// Main function that uses the default helper implementations
///
/// This function handles errors internally and prints them rather than propagating them
pub fn walk_dirs(args: &Args) {
    // Use default implementations
    let cmd_helper = DefaultCommandHelper;
    let read_val_helper = DefaultReadValHelper;

    // Call the injectable version with default implementations
    if let Err(e) = walk_dirs_with_helpers(args, &cmd_helper, &read_val_helper) {
        eprintln!("Error processing directories: {}", e);
    }
}

/// Compile regex patterns from strings
fn compile_patterns(
    patterns: &[String],
    verbose: bool,
    pattern_type: &str
) -> Result<Vec<Regex>, StartError> {
    let mut compiled_patterns = Vec::new();
    
    if !patterns.is_empty() {
        if verbose {
            crate::utils::log_utils::info(
                &format!("{} paths: {:?}", pattern_type, patterns),
                verbose as u8
            );
        }
        
        for pattern in patterns {
            let regex = Regex::new(pattern)?;
            compiled_patterns.push(regex);
        }
    }
    
    Ok(compiled_patterns)
}

/// Check if a path should be included based on patterns
fn should_include_path(
    path: &str,
    exclude_patterns: &[Regex],
    include_patterns: &[Regex],
    verbose: bool
) -> bool {
    // Check exclude patterns - skip if any match
    if !exclude_patterns.is_empty() 
        && exclude_patterns.iter().any(|pattern| pattern.is_match(path)) 
    {
        return false;
    }
    
    // Check include patterns - skip if none match
    if !include_patterns.is_empty() 
        && include_patterns.iter().all(|pattern| !pattern.is_match(path)) 
    {
        if verbose {
            crate::utils::log_utils::info(
                &format!("Skipping path as it doesn't match any include pattern: {}", path),
                verbose as u8
            );
        }
        return false;
    }
    
    true
}

/// Process a docker-compose.yml file based on mode
fn process_compose_file<C: CommandHelper, R: ReadValHelper>(
    entry: &walkdir::DirEntry,
    entry_path_str: &str,
    args: &Args,
    manager: &mut Option<RebuildManager<'_, C, R>>
) -> Result<(), StartError> {
    match args.mode {
        args::Mode::Rebuild => {
            if let Some(mgr) = manager {
                if let Err(e) = mgr.rebuild(entry, args) {
                    eprintln!("Error rebuilding from {}: {}", entry_path_str, e);
                }
            }
        }
        args::Mode::RestartSvcs => {
            drop_mgr(manager);
            match restartsvcs::restart_services(args) {
                Ok(_) => {
                    if args.verbose {
                        crate::utils::log_utils::info("Services restarted successfully", args.verbose);
                    }
                },
                Err(e) => {
                    eprintln!("Error restarting services: {}", e);
                }
            }
        }
        _ => {}
    }
    
    Ok(())
}

/// Version of walk_dirs that accepts dependency injection for testing
pub fn walk_dirs_with_helpers<C: CommandHelper, R: ReadValHelper>(
    args: &Args,
    cmd_helper: &C,
    read_val_helper: &R,
) -> Result<(), StartError> {
    // Compile patterns
    let exclude_patterns = compile_patterns(&args.exclude_path_patterns, args.verbose, "Excluding")?;
    let include_patterns = compile_patterns(&args.include_path_patterns, args.verbose, "Including")?;

    if args.verbose {
        crate::utils::log_utils::info(
            &format!("Rebuild images in path: {}", args.path.display()),
            args.verbose
        );
    }

    // Create rebuild manager
    let mut manager = Some(build::rebuild::RebuildManager::new(
        cmd_helper,
        read_val_helper,
    ));

    // Walk directory tree looking for docker-compose.yml files
    for entry in WalkDir::new(&args.path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.file_name() == "docker-compose.yml" {
            // Get path as string, safely
            let entry_path_str = match entry.path().to_str() {
                Some(path_str) => path_str,
                None => {
                    eprintln!("Skipping path with invalid UTF-8: {:?}", entry.path());
                    continue;
                }
            };
            
            // Check if this path should be included
            if !should_include_path(entry_path_str, &exclude_patterns, &include_patterns, args.verbose) {
                continue;
            }
            
            // Process the file
            process_compose_file(&entry, entry_path_str, args, &mut manager)?;
        }
    }
    
    Ok(())
}

fn drop_mgr<C: CommandHelper, R: ReadValHelper>(manager: &mut Option<RebuildManager<'_, C, R>>) {
    if let Some(manager) = manager.take() {
        mem::drop(manager);
    }
}
