use crate::args::Args;
use crate::helpers::cmd_helper_fns as cmd;
use crate::helpers::podman_helper_fns;

use chrono::{DateTime, Local};
// use regex::Regex;
use serde_yaml::Value;
use std::cmp::max;
use std::fs;
use std::fs::File;
use std::io::{self, Write};
use std::vec;
use walkdir::DirEntry;

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

pub fn rebuild(args: &Args, entry: &DirEntry, images_checked: &mut Vec<String>) {
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
    let term_width = cmd::get_terminal_display_width();
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
                format_time_ago(
                    podman_helper_fns::get_podman_image_upstream_create_time(image).unwrap()
                )
            );
            println!(
                "Pulled: {}",
                format_time_ago(podman_helper_fns::get_podman_ondisk_modify_time(image).unwrap())
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
            build_image_from_dockerfile(
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

fn read_yaml_file(file: &str) -> Value {
    let file = File::open(file).expect("file not found");
    let yaml: Value = serde_yaml::from_reader(file).expect("Error reading file");
    yaml
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
