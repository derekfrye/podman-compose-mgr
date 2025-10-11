use crate::image_build::buildfile_build;
use crate::image_build::buildfile_helpers;
use crate::interfaces::CommandHelper;
use crate::utils::build_logger::{BuildLogLevel, BuildLogger};
use thiserror::Error;
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
pub fn start<C: CommandHelper>(
    cmd_helper: &C,
    dir: &DirEntry,
    custom_img_nm: &str,
    build_args: &[&str],
    logger: &dyn BuildLogger,
) -> Result<(), BuildfileError> {
    let buildfiles = buildfile_helpers::find_buildfile(dir, custom_img_nm, build_args);
    if buildfiles.is_none()
        || buildfiles.as_ref().unwrap().is_empty()
        || buildfiles
            .as_ref()
            .unwrap()
            .iter()
            .all(|file| file.filepath.is_none())
    {
        let msg = format!(
            "No Dockerfile or Makefile found at '{}'",
            dir.path().display()
        );
        logger.log(BuildLogLevel::Warn, &msg);
        return Err(BuildfileError::RebuildError(msg));
    } else if let Some(found_buildfiles) = buildfiles {
        let build_config = crate::image_build::buildfile_helpers::read_val_loop(&found_buildfiles);

        if build_config.file.filepath.is_some() {
            buildfile_build::build_image_from_spec(cmd_helper, &build_config)
                .map_err(|e| BuildfileError::CommandExecution(Box::new(e)))?;
        }
    }
    Ok(())
}
