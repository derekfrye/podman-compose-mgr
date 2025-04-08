use crate::image_build::image::Image;
use crate::image_build::ui;
use crate::interfaces::CommandHelper;
use std::path::Path;
use walkdir::DirEntry;

/// Pull a container image using podman
///
/// # Errors
///
/// Returns an error if:
/// - The podman command fails to execute
/// - The command execution returns a non-zero exit code
pub fn pull_image<C: CommandHelper>(cmd_helper: &C, image: &str) -> Result<(), String> {
    let podman_args = vec!["pull".to_string(), image.to_string()];

    cmd_helper
        .exec_cmd("podman", podman_args)
        .map_err(|e| format!("Failed to pull image {}: {}", image, e))
}

/// Display image and container information
pub fn display_image_info<C: CommandHelper>(
    cmd_helper: &C,
    custom_img_nm: &str,
    container_name: &str,
    docker_compose_pth: &str,
    entry: &DirEntry,
) {
    // Display basic info
    ui::display_basic_image_info(custom_img_nm, container_name, docker_compose_pth);

    // Display timestamps
    ui::display_image_timestamps(custom_img_nm);

    // Get parent directory safely and display build file status
    let parent_dir = entry.path().parent().unwrap_or_else(|| Path::new("/"));
    ui::display_build_file_status(cmd_helper, parent_dir);
}

/// Check if an image should be skipped
pub fn should_skip_image(
    images_checked: &[Image],
    image_string: &str,
    container_nm_string: &str,
) -> bool {
    if images_checked.is_empty() {
        return false;
    }

    // Check if this image should be skipped by name
    let img_is_set_to_skip = images_checked.iter().any(|i| {
        if let Some(ref name) = i.name {
            name == image_string && i.skipall_by_this_name
        } else {
            false
        }
    });

    // Check if we've already processed this image+container combo
    let img_and_container_previously_reviewed = images_checked.iter().any(|i| {
        if let Some(ref name) = i.name {
            if let Some(ref container_name) = i.container {
                name == image_string && container_name == container_nm_string
            } else {
                false
            }
        } else {
            false
        }
    });

    img_is_set_to_skip || img_and_container_previously_reviewed
}
