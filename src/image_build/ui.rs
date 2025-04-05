use crate::utils::podman_utils;
use crate::build::image::format_time_ago;
use std::path::Path;
use crate::interfaces::CommandHelper;

/// Display basic image information
pub fn display_basic_image_info(custom_img_nm: &str, container_name: &str, docker_compose_pth: &str) {
    println!("Image: {}", custom_img_nm);
    println!("Container name: {}", container_name);
    println!("Compose file: {}", docker_compose_pth);
}

/// Display image timestamps
pub fn display_image_timestamps(custom_img_nm: &str) {
    // Display image creation time
    match podman_utils::get_podman_image_upstream_create_time(custom_img_nm) {
        Ok(created_time) => {
            println!("Created: {}", format_time_ago(created_time));
        },
        Err(e) => {
            println!("Created: Error getting creation time - {}", e);
        }
    }
    
    // Display image pull time
    match podman_utils::get_podman_ondisk_modify_time(custom_img_nm) {
        Ok(pull_time) => {
            println!("Pulled: {}", format_time_ago(pull_time));
        },
        Err(e) => {
            println!("Pulled: Error getting pull time - {}", e);
        }
    }
}

/// Check and display if build files exist
pub fn display_build_file_status<C: CommandHelper>(cmd_helper: &C, parent_dir: &Path) {
    // Check if Dockerfile exists
    println!(
        "Dockerfile exists: {}",
        cmd_helper.file_exists_and_readable(
            &parent_dir.join("Dockerfile")
        )
    );
    
    // Check if Makefile exists
    println!(
        "Makefile exists: {}",
        cmd_helper.file_exists_and_readable(
            &parent_dir.join("Makefile")
        )
    );
}

/// Display help information for image choices
pub fn display_help() {
    println!("p = Pull image from upstream.");
    println!("N = Do nothing, skip this image.");
    println!("d = Display info (image name, docker-compose.yml path, upstream img create date, and img on-disk modify date).");
    println!("b = Build image from the Dockerfile residing in same path as the docker-compose.yml.");
    println!("s = Skip all subsequent images with this same name (regardless of container name).");
    println!("? = Display this help.");
}