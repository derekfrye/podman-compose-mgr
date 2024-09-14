mod args;
mod rebuild;
mod helpers {
    pub mod cmd_helper_fns;
    pub mod podman_helper_fns;
}
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

    match args.mode {
        args::Mode::Rebuild | args::Mode::Secrets => walk_dirs(&args),
        // args::Mode::Secrets => secrets(&args),
        args::Mode::RestartSvcs => restartsvcs::restart_services(&args),
    }

    Ok(())
}

fn walk_dirs(args: &Args) {
    let mut exclude_patterns = Vec::new();
    let images_checked: &mut Vec<String> = &mut vec![];

    if args.exclude_path_patterns.len() > 0 {
        if args.verbose {
            println!("Excluding paths: {:?}", args.exclude_path_patterns);
        }
        for pattern in &args.exclude_path_patterns {
            exclude_patterns.push(Regex::new(pattern).unwrap());
        }
    }

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
                args::Mode::Rebuild => rebuild::rebuild(&args, &entry, images_checked),
                args::Mode::Secrets => secrets::secrets(&args, &entry),
                args::Mode::RestartSvcs => restartsvcs::restart_services(&args),
            }
        }
    }
}
