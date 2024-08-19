mod args;
mod docker_build;
mod image_cmd;
mod podman;

use args::Args;
use image_cmd::exec_cmd;
use regex::Regex;
use std::cmp::max;

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

use std::process::{Command, Stdio};
use std::vec;
use walkdir::{DirEntry, WalkDir};

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
        args::Mode::Rebuild => rebuild(&args),
        args::Mode::Secrets => secrets(&args),
    }

    Ok(())
}

fn rebuild(args: &Args) {
    // Define the pattern to match image names
    let pattern = Regex::new(r"^\s*image:\s*([a-zA-Z0-9_\.\-/:]+)").unwrap();
    // Update the pattern to match the required image format with tags
    // let djf_pattern = Regex::new(r"^djf/[\w]+(:[\w][\w.-]{0,127})?$").unwrap();
    // let djf_pattern = Regex::new(r"^\s*djf/[\w\d:._-]+$").unwrap();

    if args.verbose {
        println!("Rebuild images in path: {}", args.path.display());
    }

    let mut exclude_patterns = Vec::new();
    let mut images_checked = vec![];

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
            if let Ok(file) = File::open(entry.path()) {
                let reader = BufReader::new(file);
                for line_orig in reader.lines() {
                    let line = line_orig.unwrap();
                    if let Some(captures) = pattern.captures(&line) {
                        for x in 0..captures.len() {
                            let image = captures.get(x).unwrap().as_str().trim().to_string();
                            // don't use x = 0, becuase that's the full image: xyz/xyx:latest
                            if x != 0 && !images_checked.contains(&image) {
                                read_val_from_cmd_line_and_proceed(
                                    &entry,
                                    &image,
                                    args.build_args.clone(),
                                );
                                images_checked.push(image);
                            }
                        }
                    }
                }
            }
        }
    }

    if args.verbose {
        println!("Done.");
    }
}

fn read_val_from_cmd_line_and_proceed(entry: &DirEntry, image: &str, build_args: Vec<String>) {
    let docker_compose_pth = entry
        .path()
        .parent()
        .unwrap_or(std::path::Path::new("/"))
        .display();

    let docker_compose_pth_fmtted = format!("{}", docker_compose_pth);
    let refresh_static = format!("Refresh  from ? p/N/d/b/?: ");
    let refresh_prompt = format!(
        "Refresh {} from {}? p/N/d/b/?: ",
        image, docker_compose_pth_fmtted
    );

    // if the prompt is too long, we need to shorten some stuff.
    //
    // At a minimum, we'll display our 23 chars of "refresh ... from ?" stuff.
    // Then we divide remaining space equally between image name and path name.
    // We're not going to go less than 12 chars for path and image name, anything less feels like we're cutting too much off maybe.
    // This means total display chars is 23 + 12 + 12 = 47 at a min
    // if user has less than 47 wide, well then we'll have to let the terminal word-wrap.
    let term_width = get_terminal_display_width();
    // println!("term_width: {}", term_width);
    // println!("refresh_prompt len: {}", refresh_prompt.len());
    let mut docker_compose_pth_shortened = docker_compose_pth_fmtted.to_string();
    // let docker_compose_path_orig = docker_compose_pth_shortened.to_string();
    let mut image_shortened = image.to_string();
    // let image_orig = image.to_string();
    // 1 char for a little buffer so it doesnt wrap after user input
    if refresh_prompt.len() > term_width - 1 {
        let truncated_symbols = "...";
        let mut max_avail_chars_for_image_and_path =
            max(term_width, 47) - refresh_static.len() - (2 * truncated_symbols.len()) - 1;
        if max_avail_chars_for_image_and_path % 2 != 0 {
            max_avail_chars_for_image_and_path -= 1;
        }

        if docker_compose_pth_shortened.len() > max_avail_chars_for_image_and_path / 2 {
            docker_compose_pth_shortened = format!(
                "...{}",
                docker_compose_pth_shortened
                    [docker_compose_pth_shortened.len() - max_avail_chars_for_image_and_path / 2..]
                    .to_string()
            );
        }

        if image_shortened.len() > max_avail_chars_for_image_and_path / 2 {
            image_shortened = format!(
                "...{}",
                image_shortened[image_shortened.len() - max_avail_chars_for_image_and_path / 2..]
                    .to_string()
            );
        }
    }
    // make sure this str matches str refresh_prompt above or the wrap logic above breaks
    // also, this same string is also used near end of this loop, make sure it matches there too
    print!(
        "Refresh {} from {}? p/N/d/b/?: ",
        image_shortened, docker_compose_pth_shortened
    );
    loop {
        let mut input = String::new();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        if input.eq_ignore_ascii_case("p") {
            // Pull the image using podman and stream the output
            pull_it(image);
            break;
        } else if input.eq_ignore_ascii_case("d") {
            println!("Image: {}", image);
            println!("Compose file: {}", docker_compose_pth_fmtted);
            println!(
                "Created: {}",
                podman::format_time_ago(
                    podman::get_podman_image_upstream_create_time(image).unwrap()
                )
            );
            println!(
                "Pulled: {}",
                podman::format_time_ago(podman::get_podman_ondisk_modify_time(image).unwrap())
            );
            print!(
                "Refresh {} from {}? p/N/d/b/?: ",
                image_shortened, docker_compose_pth_shortened
            );
        } else if input.eq_ignore_ascii_case("?") {
            println!("p = Pull image from upstream.");
            println!("N = Do nothing, skip this image.");
            println!("d = Display info (image name, docker-compose.yml path, upstream img create date, and img on-disk modify date).");
            println!("b = Build image from the Dockerfile residing in same path as the docker-compose.yml.");
            println!("? = Display this help.");
            print!(
                "Refresh {} from {}? p/N/d/b/?: ",
                image_shortened, docker_compose_pth_shortened
            );
        } else if input.eq_ignore_ascii_case("b") {
            docker_build::build_image_from_dockerfile(
                entry,
                image,
                build_args.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
            );
            break;
        } else {
            break;
        }
    }
}

fn get_terminal_display_width() -> usize {
    let (width, _) = term_size::dimensions().unwrap_or((80, 24));
    width
}

fn pull_it(image: &str) {
    let mut x = vec![];

    x.push("pull");
    x.push(image);
    exec_cmd("podman", x);
}

fn secrets(args: &Args) {
    // Define the pattern to match secrets
    let pattern = Regex::new(r"^\s*-\s*([\w\d_]+)").unwrap();

    for entry in WalkDir::new(&args.path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.file_name() == "docker-compose.yml" {
            if let Ok(file) = File::open(entry.path()) {
                for line in io::BufReader::new(file).lines() {
                    if let Ok(line) = line {
                        if let Some(captures) = pattern.captures(&line) {
                            let secret = captures.get(1).unwrap().as_str().trim();
                            // Check if the secret exists
                            let secret_path = Path::new(&args.path).join("secrets").join(secret);
                            if !secret_path.exists() {
                                let z = entry.path().parent().unwrap_or(Path::new("/")).display();
                                let mut z_display = format!("{}", z);
                                let z_len = z_display.len();
                                if z_len > 25 {
                                    let start = &z_display[..5];
                                    let end = &z_display[z_len - 20..];
                                    z_display = format!("{}...{}", start, end);
                                }
                                print!("Create secret '{}' for {}? (y/N): ", secret, z_display);
                                io::stdout().flush().unwrap();
                                let mut input = String::new();
                                io::stdin().read_line(&mut input).unwrap();
                                let input = input.trim();
                                if input.eq_ignore_ascii_case("y") {
                                    // Create the secret using podman and stream the output
                                    println!("Creating secret: {}", secret);
                                    let mut child = Command::new("podman")
                                        .arg("secrets")
                                        .arg("create")
                                        .arg(secret)
                                        .arg(secret_path)
                                        .stdout(Stdio::piped())
                                        .spawn()
                                        .expect("Failed to execute podman secrets create");

                                    if let Some(stdout) = child.stdout.take() {
                                        let reader = BufReader::new(stdout);
                                        for line in reader.lines() {
                                            if let Ok(line) = line {
                                                println!("{}", line);
                                            }
                                        }
                                    }

                                    let _ = child.wait().expect("Command wasn't running");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
