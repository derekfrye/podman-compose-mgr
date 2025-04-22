// Public modules
mod initialization;
mod types;
mod validators;

// Re-export everything from the submodules
pub use initialization::*;
pub use types::*;
pub use validators::*;

use clap::Parser;
use std::process;

/// Parse command line arguments and perform validation with processing
///
/// This function:
/// 1. Parses command line arguments
/// 2. For SecretInitialize mode, processes the init filepath if needed
/// 3. Returns the validated Args structure
///
/// # Returns
///
/// * `Args` - The validated arguments
///
/// # Panics
///
/// Panics if validation fails
pub fn args_checks() -> Args {
    let mut args = Args::parse();

    // Process and validate the arguments
    if let Err(e) = args.validate_and_process() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    args
}
