pub mod azure;
pub mod b2_storage;
pub mod error;
pub mod file_details;
pub mod initialize;
pub mod models;
pub mod r2_storage;
pub mod s3_storage_base;
pub mod upload;
pub mod upload_utils;
pub mod user_prompt;
pub mod utils;
pub mod validation;

use crate::Args;
use crate::secrets::error::Result;

/// Process secrets mode
///
/// Handles the different secret-related modes.
pub fn process_secrets_mode(args: &Args) -> Result<()> {
    match args.mode {
        
        crate::args::Mode::SecretRetrieve => {
            validation::validate(args)?;
        }
        crate::args::Mode::SecretInitialize => {
            initialize::process(args)?;
        }
        crate::args::Mode::SecretUpload => {
            upload::process(args)?;
        }
        _ => {
            return Err(Box::<dyn std::error::Error>::from(
                "Unsupported mode for secrets processing",
            ));
        }
    }
    Ok(())
}
