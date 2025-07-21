// Public modules
pub mod types;
mod validators;

// Re-export everything from the submodules
pub use types::*;
pub use validators::*;

use clap::Parser;
use std::process;

/// Parse command line arguments and perform validation with processing
///
/// This function:
/// 1. Parses command line arguments
/// 2. For `SecretInitialize` mode, processes the init filepath if needed
/// 3. Returns the validated Args structure
///
/// # Returns
///
/// * `Args` - The validated arguments
///
/// # Panics
///
/// Panics if validation fails
#[must_use] pub fn args_checks() -> Args {
    let args = Args::parse();

    // Validate the arguments
    if let Err(e) = args.validate() {
        eprintln!("Error: {e}");
        process::exit(1);
    }

    args
}
