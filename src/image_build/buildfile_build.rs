use crate::utils::cmd_utils as cmd;
use crate::utils::podman_utils;
use crate::image_build::buildfile_error::BuildfileError;
use crate::image_build::buildfile_types::{BuildChoice, WhatWereBuilding};

/// Build an image from a dockerfile
pub fn build_dockerfile_image(build_config: &WhatWereBuilding) -> Result<(), BuildfileError> {
    let _ = podman_utils::pull_base_image(build_config.file.filepath.as_ref().unwrap());
    
    let dockerfile_path = build_config
        .file
        .filepath
        .as_ref()
        .unwrap()
        .to_str()
        .unwrap();
        
    let mut podman_args = vec![
        "build",
        "-t",
        build_config.file.custom_img_nm.as_ref().unwrap(),
        "-f",
        dockerfile_path,
    ];
    
    // Add build args
    for arg in build_config.file.build_args.iter() {
        podman_args.push("--build-arg");
        podman_args.push(arg);
    }
    
    podman_args.push(build_config.file.parent_dir.to_str().unwrap());
    
    cmd::exec_cmd("podman", &podman_args[..]).map_err(BuildfileError::from)
}

/// Build an image using a makefile
pub fn build_makefile_image(build_config: &WhatWereBuilding) -> Result<(), BuildfileError> {
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
    } else {
        build_config.file.parent_dir.to_str().unwrap()
    };
    
    cmd::exec_cmd("make", &["-C", chg_dir, "clean"])?;
    Ok(cmd::exec_cmd("make", &["-C", chg_dir])?)
}

/// Build an image from the specified configuration
pub fn build_image_from_spec(build_config: WhatWereBuilding) -> Result<(), BuildfileError> {
    match build_config.file.filetype {
        BuildChoice::Dockerfile => build_dockerfile_image(&build_config),
        BuildChoice::Makefile => build_makefile_image(&build_config),
    }
}