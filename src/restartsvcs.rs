use std::error::Error;
use crate::args::Args;
use crate::utils::cmd_utils as cmd;

/// Restart services managed by podman-compose
///
/// # Arguments
/// * `args` - Command-line arguments
///
/// # Errors
///
/// Returns an error if the podman command fails
pub fn restart_services(args: &Args) -> Result<(), Box<dyn Error>> {
    if args.verbose {
        println!("Starting {}...", args.path.display());
    }
    
    let podman_args = ["restart", "-f", "docker-compose.yml"];

    cmd::exec_cmd("podman", &podman_args)?;
    
    Ok(())
}
