use crate::interfaces::CommandHelper;

use super::errors::RebuildError;

/// Pull a container image using podman
///
/// # Errors
///
/// Returns an error if:
/// - The podman command fails to execute
/// - The command execution returns a non-zero exit code
pub fn pull_image<C: CommandHelper>(cmd_helper: &C, image: &str) -> Result<(), RebuildError> {
    let podman_args = vec!["pull".to_string(), image.to_string()];

    cmd_helper
        .exec_cmd("podman", podman_args)
        .map_err(|e| RebuildError::CommandExecution(format!("Failed to pull image {image}: {e}")))
}
