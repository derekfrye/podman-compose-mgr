use crate::errors::PodmanComposeMgrError;
use crate::image_build::buildfile_types::{BuildChoice, WhatWereBuilding};
use crate::interfaces::CommandHelper;

/// Build an image from a dockerfile
///
/// # Arguments
///
/// * `build_config` - Configuration for the build process
///
/// # Returns
///
/// * `Result<(), PodmanComposeMgrError>` - Success or error
///
/// # Errors
///
/// Returns an error if the build process fails or if required fields are missing.
///
/// # Panics
///
/// Panics if build configuration contains invalid paths or if `unwrap()` fails on expected values.
pub fn build_dockerfile_image<C: CommandHelper>(
    cmd_helper: &C,
    build_config: &WhatWereBuilding,
) -> Result<(), PodmanComposeMgrError> {
    if let Some(path) = build_config.file.filepath.as_ref() {
        let _ = cmd_helper.pull_base_image(path);
    }

    let dockerfile_path = build_config
        .file
        .filepath
        .as_ref()
        .unwrap()
        .to_str()
        .unwrap();

    let mut podman_args = vec![
        "build".to_string(),
        "-t".to_string(),
        build_config.file.custom_img_nm.as_ref().unwrap().clone(),
        "-f".to_string(),
        dockerfile_path.to_string(),
    ];

    for arg in &build_config.file.build_args {
        podman_args.push("--build-arg".to_string());
        podman_args.push(arg.clone());
    }

    podman_args.push(build_config.file.parent_dir.to_str().unwrap().to_string());

    cmd_helper
        .exec_cmd("podman", podman_args)
        .map_err(PodmanComposeMgrError::from)
}

/// Build an image using a makefile
///
/// # Arguments
///
/// * `build_config` - Configuration for the build process
///
/// # Returns
///
/// * `Result<(), PodmanComposeMgrError>` - Success or error
///
/// # Errors
///
/// Returns an error if the makefile execution fails or if required fields are missing.
///
/// # Panics
///
/// Panics if build configuration contains invalid paths or if `unwrap()` fails on expected values.
pub fn build_makefile_image<C: CommandHelper>(
    cmd_helper: &C,
    build_config: &WhatWereBuilding,
) -> Result<(), PodmanComposeMgrError> {
    let chg_dir = if build_config.follow_link {
        build_config
            .file
            .link_target_dir
            .as_ref()
            .unwrap()
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    } else {
        build_config.file.parent_dir.to_str().unwrap().to_string()
    };

    cmd_helper
        .exec_cmd(
            "make",
            vec!["-C".to_string(), chg_dir.clone(), "clean".to_string()],
        )
        .map_err(PodmanComposeMgrError::from)?;

    cmd_helper
        .exec_cmd("make", vec!["-C".to_string(), chg_dir])
        .map_err(PodmanComposeMgrError::from)
}

/// Build an image from the specified configuration
///
/// # Arguments
///
/// * `build_config` - Configuration specifying how to build the image
///
/// # Returns
///
/// * `Result<(), PodmanComposeMgrError>` - Success or error
///
/// # Errors
///
/// Returns an error if the build process fails, depending on the build type (Dockerfile or Makefile).
pub fn build_image_from_spec<C: CommandHelper>(
    cmd_helper: &C,
    build_config: &WhatWereBuilding,
) -> Result<(), PodmanComposeMgrError> {
    match build_config.file.filetype {
        BuildChoice::Dockerfile => build_dockerfile_image(cmd_helper, build_config),
        BuildChoice::Makefile => build_makefile_image(cmd_helper, build_config),
    }
}
