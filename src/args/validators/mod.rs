pub mod dir_validators;
pub mod file_validators;

use super::types::Args;

// Re-export commonly used validators
pub use dir_validators::{check_readable_dir, check_readable_dir_path, check_writable_dir};
pub use file_validators::{
    check_file_writable, check_file_writable_path, check_readable_file, check_readable_path,
    check_valid_json_file, check_valid_json_path,
};

/// Validate the args for rebuild mode
///
/// # Errors
///
/// Returns an error if the arguments are invalid for rebuild mode.
pub fn validate(_args: &Args) -> Result<(), String> {
    // For rebuild mode, basic validation is handled by clap value_parser
    // No additional validation needed
    Ok(())
}
