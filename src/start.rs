use std::mem;

use regex::Regex;
use walkdir::WalkDir;

use crate::{args, build::{self, rebuild::RebuildManager}, restartsvcs, Args};

pub fn walk_dirs(args: &Args) {
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
