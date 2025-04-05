use podman_compose_mgr::{
    args::{self, Mode},
    secrets,
    start::walk_dirs,
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
        Mode::SecretRefresh | Mode::SecretRetrieve => {
            if let Err(e) = secrets::process_secrets_mode(&args) {
                eprintln!("Error processing secrets: {}", e);
                std::process::exit(1);
            }
        },
        Mode::RestartSvcs => {
            // This is a special test mode for Azure KeyVault
            // Using RestartSvcs as a stand-in for testing Azure connection
            if let Err(e) = secrets::test_azure_connection(&args) {
                eprintln!("Error testing Azure connection: {}", e);
                std::process::exit(1);
            }
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
