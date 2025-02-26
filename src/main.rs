mod args;
mod build {
    pub mod build;
    pub mod rebuild;
}
mod helpers {
    pub mod cmd_helper_fns;
    pub mod podman_helper_fns;
}
mod read_val;
mod restartsvcs;
mod secrets;

use args::Args;
use build::rebuild::RebuildManager;
use regex::Regex;
// use futures::executor;
use std::{io, mem};
use walkdir::WalkDir;

fn main() -> io::Result<()> {
    // Parse command-line arguments
    let args = args::args_checks();
    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    match args.mode {
        args::Mode::SecretRefresh => {
            if let Err(e) = secrets::update_mode(&args) {
                eprintln!("Error refreshing secrets: {}", e);
            }
        }
        args::Mode::SecretRetrieve => {
            if let Err(e) = secrets::validate(&args) {
                eprintln!("Error retrieving secrets: {}", e);
            }
        }
        _ => {
            walk_dirs(&args);
        }
    }

    if args.verbose {
        println!("Done.");
    }

    Ok(())
}

fn walk_dirs(args: &Args) {
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

    let mut manager: Option<RebuildManager> = Some(build::rebuild::RebuildManager::new());

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
                    // let mut manager = rebuild::RebuildManager::new();
                    if let Some(ref mut manager) = manager {
                        manager.rebuild(&entry, args);
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

fn drop_mgr(manager: &mut Option<RebuildManager>) {
    if let Some(manager) = manager.take() {
        mem::drop(manager);
    }
}
