pub mod discovery;
pub mod prompt;
pub mod types;

use crate::image_build::buildfile_build;
use crate::image_build::buildfile_types::{
    BuildChoice as BuildChoiceExternal, BuildFile as BuildFileExternal,
    WhatWereBuilding as WhatWereBuildingExternal,
};
use discovery::find_buildfile;
use prompt::read_val_loop;
use thiserror::Error;
use types::BuildChoice;
use walkdir::DirEntry;

#[derive(Debug, Error)]
pub enum BuildfileError {
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("Path contains invalid UTF-8: {0}")]
    InvalidPath(String),

    #[error("Rebuild error: {0}")]
    RebuildError(String),

    #[error("Command execution error: {0}")]
    CommandExecution(#[from] Box<dyn std::error::Error>),
}

/// Start the build process for a directory
///
/// # Arguments
///
/// * `dir` - Directory entry to search for build files
/// * `custom_img_nm` - Custom image name to use
/// * `build_args` - Build arguments to pass to the build process
///
/// # Returns
///
/// * `Result<(), BuildfileError>` - Success or error
///
/// # Errors
///
/// Returns an error if no build files are found or if the build process fails.
///
/// # Panics
///
/// Panics if build files cannot be processed or if internal state is invalid.
pub fn start(
    dir: &DirEntry,
    custom_img_nm: &str,
    build_args: &[&str],
) -> Result<(), BuildfileError> {
    let buildfiles = find_buildfile(dir, custom_img_nm, build_args);
    if buildfiles.is_none()
        || buildfiles.as_ref().unwrap().is_empty()
        || buildfiles
            .as_ref()
            .unwrap()
            .iter()
            .all(|file| file.filepath.is_none())
    {
        eprintln!(
            "No Dockerfile or Makefile found at '{}'",
            dir.path().display()
        );
    } else if let Some(found_buildfiles) = buildfiles {
        let build_config = read_val_loop(&found_buildfiles);

        if build_config.file.filepath.is_some() {
            // Convert internal types to external types
            let external_build_config = WhatWereBuildingExternal {
                file: BuildFileExternal {
                    filetype: match build_config.file.filetype {
                        BuildChoice::Dockerfile => BuildChoiceExternal::Dockerfile,
                        BuildChoice::Makefile => BuildChoiceExternal::Makefile,
                    },
                    filepath: build_config.file.filepath.clone(),
                    parent_dir: build_config.file.parent_dir.clone(),
                    link_target_dir: build_config.file.link_target_dir.clone(),
                    base_image: build_config.file.base_image.clone(),
                    custom_img_nm: build_config.file.custom_img_nm.clone(),
                    build_args: build_config.file.build_args.clone(),
                },
                follow_link: build_config.follow_link,
            };

            buildfile_build::build_image_from_spec(&external_build_config)
                .map_err(|e| BuildfileError::CommandExecution(Box::new(e)))?;
        }
    }
    Ok(())
}
