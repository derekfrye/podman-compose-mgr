pub mod azure;
pub mod error;
pub mod file_details;
pub mod initialize;
pub mod json_utils;
pub mod models;
pub mod r2_storage;
pub mod s3;
pub mod s3_storage_base;
pub mod upload;
pub mod upload_handlers;
pub mod upload_utils;
pub mod upload_prompt;
pub mod retrieve_prompt;
pub mod user_prompt;
pub mod utils;
pub mod validation;

use crate::Args;
use crate::secrets::error::Result;
use crate::utils::log_utils::Logger;

/// Process secrets mode
///
/// Handles the different secret-related modes.
pub fn process_secrets_mode(args: &Args, logger: &Logger) -> Result<()> {
    match args.mode {
        crate::args::Mode::SecretRetrieve => {
            validation::validate(args, logger)?;
        }
        crate::args::Mode::SecretInitialize => {
            initialize::process(args, logger)?;
        }
        crate::args::Mode::SecretUpload => {
            upload::process(args, logger)?;
        }
        _ => {
            return Err(Box::<dyn std::error::Error>::from(
                "Unsupported mode for secrets processing",
            ));
        }
    }
    Ok(())
}
