pub mod azure;
pub mod error;
pub mod models;
pub mod prompt;
pub mod utils;
pub mod validation;

use crate::Args;
use crate::secrets::error::Result;

/// Process secrets mode
///
/// Handles the different secret-related modes.
pub fn process_secrets_mode(args: &Args) -> Result<()> {
    match args.mode {
        crate::args::Mode::SecretRefresh => {
            azure::update_mode(args)?;
        }
        crate::args::Mode::SecretRetrieve => {
            validation::validate(args)?;
        }
        _ => {
            return Err(Box::<dyn std::error::Error>::from("Unsupported mode for secrets processing"));
        }
    }
    Ok(())
}