mod args;
mod rebuild;
mod helpers {
    pub mod cmd_helper_fns;
    pub mod podman_helper_fns;
}
mod read_val;
mod restartsvcs;
mod secrets;

use args::Args;
use regex::Regex;
use std::io;
use walkdir::WalkDir;

fn main() -> io::Result<()> {
    // Parse command-line arguments
    let args = args::args_checks();

    // if args.verbose {
    //     println!("Path: {}", args.path);
    //     println!("Mode: {:?}", args.mode);
    //     if let Some(secrets_file) = &args.secrets_file {
    //         println!("Secrets file: {}", secrets_file.display());
    //     }
    // }

    walk_dirs(&args);

    Ok(())
}

fn walk_dirs(args: &Args) {
    let mut exclude_patterns = Vec::new();

    if args.exclude_path_patterns.len() > 0 {
        if args.verbose {
            println!("Excluding paths: {:?}", args.exclude_path_patterns);
        }
        for pattern in &args.exclude_path_patterns {
            exclude_patterns.push(Regex::new(pattern).unwrap());
        }
    }

    if args.verbose {
        println!("Rebuild images in path: {}", args.path.display());
    }

    let mut manager = rebuild::RebuildManager::new();

    for entry in WalkDir::new(&args.path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.file_name() == "docker-compose.yml" {
            if exclude_patterns.len() > 0
                && exclude_patterns
                    .iter()
                    .any(|pattern| pattern.is_match(entry.path().to_str().unwrap()))
            {
                continue;
            }
            match args.mode {
                args::Mode::Rebuild => {
                    // let mut manager = rebuild::RebuildManager::new();
                    manager.rebuild(&entry, &args);

                    // rebuild::rebuild(&args, &entry, );
                }
                args::Mode::Secrets => secrets::secrets(&args, &entry),
                args::Mode::RestartSvcs => restartsvcs::restart_services(&args),
            }
        }
    }

    if args.verbose {
        println!("Done.");
    }
}
