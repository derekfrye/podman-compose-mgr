use std::mem;

use regex::Regex;
use thiserror::Error;
use walkdir::WalkDir;

use crate::{
    Args, args,
    image_build::{self as build, rebuild::RebuildManager},
    interfaces::{CommandHelper, DefaultCommandHelper, DefaultReadValHelper, ReadValHelper},
    
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

/// Version of walk_dirs that accepts dependency injection for testing
pub fn walk_dirs_with_helpers<C: CommandHelper, R: ReadValHelper>(
    args: &Args,
    cmd_helper: &C,
    read_val_helper: &R,
) -> Result<(), StartError> {
    let mut exclude_patterns = Vec::new();
    let mut include_patterns = Vec::new();

    // Compile exclude patterns
    if !args.exclude_path_patterns.is_empty() {
        if args.verbose {
            println!("Excluding paths: {:?}", args.exclude_path_patterns);
        }
        
        for pattern in &args.exclude_path_patterns {
            let regex = Regex::new(pattern)?;
            exclude_patterns.push(regex);
        }
    }
    
    // Compile include patterns
    if !args.include_path_patterns.is_empty() {
        if args.verbose {
            println!("Including paths: {:?}", args.include_path_patterns);
        }
        
        for pattern in &args.include_path_patterns {
            let regex = Regex::new(pattern)?;
            include_patterns.push(regex);
        }
    }

    if args.verbose {
        println!("Rebuild images in path: {}", args.path.display());
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
            
            // Check exclude patterns - skip if any match
            if !exclude_patterns.is_empty() 
                && exclude_patterns.iter().any(|pattern| pattern.is_match(entry_path_str)) 
            {
                // if args.verbose {
                //     println!("Excluding path due to exclude pattern: {}", entry_path_str);
                // }
                continue;
            }
            
            // Check include patterns - skip if none match
            if !include_patterns.is_empty() 
                && include_patterns.iter().all(|pattern| !pattern.is_match(entry_path_str)) 
            {
                if args.verbose {
                    println!("Skipping path as it doesn't match any include pattern: {}", entry_path_str);
                }
                continue;
            }
            
            // Process according to mode
            match args.mode {
                args::Mode::Rebuild => {
                    if let Some(ref mut mgr) = manager {
                        if let Err(e) = mgr.rebuild(&entry, args) {
                            eprintln!("Error rebuilding from {}: {}", entry_path_str, e);
                        }
                    }
                }
                _ => {drop_mgr(&mut manager);}
            }
        }
    }
    
    Ok(())
}

fn drop_mgr<C: CommandHelper, R: ReadValHelper>(manager: &mut Option<RebuildManager<'_, C, R>>) {
    if let Some(manager) = manager.take() {
        mem::drop(manager);
    }
}
