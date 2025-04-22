use dockerfile_parser::Dockerfile;
use std::io::{BufReader, Read};
use std::path::Path;
use terminal_size::{self, Width};

/// Parse Dockerfile and pull base image
pub fn pull_base_image(dockerfile: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    // Use the error utils to handle file opening errors
    let file = std::fs::File::open(dockerfile).map_err(|e| {
        crate::utils::error_utils::into_boxed_error(
            e,
            &format!("Failed to open Dockerfile: {}", dockerfile.display()),
        )
    })?;

    let mut reader = BufReader::new(file);

    let mut content = String::new();
    reader.read_to_string(&mut content).map_err(|e| {
        crate::utils::error_utils::into_boxed_error(e, "Failed to read Dockerfile contents")
    })?;

    let dockerfile = Dockerfile::parse(&content).map_err(|e| {
        crate::utils::error_utils::into_boxed_error(e, "Failed to parse Dockerfile")
    })?;

    let from_img = dockerfile.instructions;
    let image_name = from_img
        .iter()
        .find_map(|instruction| {
            if let dockerfile_parser::Instruction::From(image, ..) = instruction {
                Some(image.image.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| crate::utils::error_utils::new_error("No base image found in Dockerfile"))?;

    // Use the new command utilities
    // Convert SpannedString to regular String for the command args
    let image_name_str = image_name.to_string();
    crate::utils::cmd_utils::run_command_checked("podman", &["pull", &image_name_str])
}

/// exists(), is_file() traversing links, and metadata.is_ok() traversing links
pub fn file_exists_and_readable(file: &Path) -> bool {
    match file.try_exists() {
        Ok(true) => file.is_file() && file.metadata().is_ok(),
        _ => false,
    }
}

pub fn get_terminal_display_width(specify_size: Option<usize>) -> usize {
    if let Some(size) = specify_size {
        return size;
    }
    let size = terminal_size::terminal_size();
    if let Some((Width(w), _)) = size {
        w as usize
    } else {
        80
    }
}
