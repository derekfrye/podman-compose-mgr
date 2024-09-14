use crate::args::Args;
use crate::helpers::cmd_helper_fns as cmd;
use crate::helpers::podman_helper_fns;

use chrono::{DateTime, Local};
use regex::Regex;
use serde_yaml::Value;
use std::cmp::max;
use std::fs;
use std::fs::File;
use std::io::{self, Write};
use std::vec;
use walkdir::{DirEntry, WalkDir};

fn build_image_from_dockerfile(dir: &DirEntry, image_name: &str, build_args: Vec<&str>) {
    let mut dockerfile = dir.path().to_path_buf().parent().unwrap().to_path_buf();
    dockerfile.push("Dockerfile");

    if !dockerfile.is_file()
        || !fs::metadata(&dockerfile).is_ok()
        || !fs::File::open(&dockerfile).is_ok()
    {
        eprintln!("No Dockerfile found at '{}'", dockerfile.display());
        std::process::exit(1);
    }

    let _ = cmd::pull_base_image(&dockerfile);

    let z = dockerfile.display().to_string();

    let mut x = vec![];
    x.push("build");
    x.push("-t");
    x.push(image_name);
    x.push("-f");
    x.push(&z);

    // let mut abc = string::String::new();
    for arg in build_args {
        x.push("--build-arg");
        x.push(&arg);
    }

    cmd::exec_cmd("podman", x);
}

pub fn rebuild(args: &Args, entry: &DirEntry) {
    if args.verbose {
        println!("Rebuild images in path: {}", args.path.display());
    }

            let yaml = read_yaml_file(entry.path().to_str().unwrap());
            if let Some(services) = yaml.get("services") {
                if let Some(services_map) = services.as_mapping() {
                    for (_, service_config) in services_map {
                        // println!("Service: {:?}", service_name);
                        if let Some(image) = service_config.get("image") {
                            // println!("  Image: {:?}", image);
                            if !images_checked.contains(&image.as_str().unwrap().to_string()) {
                                read_val_from_cmd_line_and_proceed(
                                    &entry,
                                    &image.as_str().unwrap().to_string(),
                                    args.build_args.clone(),
                                );
                                images_checked.push(image.as_str().unwrap().to_string());
                            }
                        }
                    }

        }
    }

    if args.verbose {
        println!("Done.");
    }
}

fn read_yaml_file(file: &str) -> Value {
    let file = File::open(file).expect("file not found");
    let yaml: Value = serde_yaml::from_reader(file).expect("Error reading file");
    yaml
}



fn get_terminal_display_width() -> usize {
    let (width, _) = term_size::dimensions().unwrap_or((80, 24));
    width
}

fn pull_it(image: &str) {
    let mut x = vec![];

    x.push("pull");
    x.push(image);
    cmd::exec_cmd("podman", x);
}

fn format_time_ago(dt: DateTime<Local>) -> String {
    let now = Local::now();
    let duration = now.signed_duration_since(dt);
    let days = duration.num_days();
    let hours = duration.num_hours();
    let minutes = duration.num_minutes();
    let seconds = duration.num_seconds();
    if days > 0 {
        format!("{} days ago", days)
    } else if hours > 0 {
        format!("{} hours ago", hours)
    } else if minutes > 0 {
        format!("{} minutes ago", minutes)
    } else {
        format!("{} seconds ago", seconds)
    }
}
