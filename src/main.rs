use podman_compose_mgr::{
    args::{self, Mode},
    secrets,
    walk_dirs::walk_dirs,
};

// use futures::executor;
use std::io;

fn main() -> io::Result<()> {
    // Parse command-line arguments
    let args = args::args_checks();
    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    match args.mode {
        Mode::SecretRefresh | Mode::SecretRetrieve | Mode::SecretInitialize | Mode::SecretUpload => {
            if let Err(e) = secrets::process_secrets_mode(&args) {
                eprintln!("Error processing secrets: {}", e);
                std::process::exit(1);
            }
        },
        Mode::RestartSvcs => {
            // Placeholder for service restart functionality
            eprintln!("Restart services mode not yet implemented");
        },
        _ => {
            walk_dirs(&args);
        }
    }

    if args.verbose {
        println!("Done.");
    }

    Ok(())
}
