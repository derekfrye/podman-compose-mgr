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
use walkdir::WalkDir;
use std::io::{self, Write};

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
        args::Mode::Rebuild => rebuild::rebuild(&args),
        args::Mode::Secrets => secrets(&args),
        args::Mode::RestartSvcs => restartsvcs::restart_services(&args),
    }

    Ok(())
}

fn walk_dirs(args: &Args  ){

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
            match  args.mode {
                args::Mode::Rebuild => rebuild::rebuild(&args, &entry),   
                args::Mode::Secrets => secrets(&args),
                args::Mode::RestartSvcs => restartsvcs::restart_services(&args),
            }
}}
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