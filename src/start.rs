use std::mem;

use regex::Regex;
use walkdir::WalkDir;

use crate::{
    Args, args,
    build::{self, rebuild::RebuildManager},
    interfaces::{CommandHelper, DefaultCommandHelper, DefaultReadValHelper, ReadValHelper},
    restartsvcs,
};

/// Main function that uses the default helper implementations
pub fn walk_dirs(args: &Args) {
    // Use default implementations
    let cmd_helper = DefaultCommandHelper;
    let read_val_helper = DefaultReadValHelper;

    // Call the injectable version with default implementations
    walk_dirs_with_helpers(args, &cmd_helper, &read_val_helper);
}

/// Version of walk_dirs that accepts dependency injection for testing
pub fn walk_dirs_with_helpers<C: CommandHelper, R: ReadValHelper>(
    args: &Args,
    cmd_helper: &C,
    read_val_helper: &R,
) {
    let mut exclude_patterns = Vec::new();
    let mut include_patterns = Vec::new();

    if !args.exclude_path_patterns.is_empty() {
        if args.verbose {
            println!("Excluding paths: {:?}", args.exclude_path_patterns);
        }
        for pattern in &args.exclude_path_patterns {
            exclude_patterns.push(Regex::new(pattern).unwrap());
        }
    }
    if !args.include_path_patterns.is_empty() {
        if args.verbose {
            println!("Including paths: {:?}", args.include_path_patterns);
        }
        for pattern in &args.include_path_patterns {
            include_patterns.push(Regex::new(pattern).unwrap());
        }
    }

    if args.verbose {
        println!("Rebuild images in path: {}", args.path.display());
    }

    let mut manager = Some(build::rebuild::RebuildManager::new(
        cmd_helper,
        read_val_helper,
    ));

    for entry in WalkDir::new(&args.path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.file_name() == "docker-compose.yml" {
            if !exclude_patterns.is_empty()
                && exclude_patterns
                    .iter()
                    .any(|pattern| pattern.is_match(entry.path().to_str().unwrap()))
            {
                continue;
            }
            if !include_patterns.is_empty()
                && include_patterns
                    .iter()
                    .any(|pattern| !pattern.is_match(entry.path().to_str().unwrap()))
            {
                continue;
            }
            match args.mode {
                args::Mode::Rebuild => {
                    if let Some(ref mut manager) = manager {
                        if let Err(e) = manager.rebuild(&entry, args) {
                            eprintln!("Error rebuilding: {}", e);
                        }
                    }
                }
                args::Mode::RestartSvcs => {
                    drop_mgr(&mut manager);
                    restartsvcs::restart_services(args);
                }
                _ => {}
            }
        }
    }
}

fn drop_mgr<C: CommandHelper, R: ReadValHelper>(manager: &mut Option<RebuildManager<'_, C, R>>) {
    if let Some(manager) = manager.take() {
        mem::drop(manager);
    }
}
