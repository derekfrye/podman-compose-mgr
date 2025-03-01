use podman_compose_mgr::{
    args::{self},
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
        args::Mode::SecretRefresh => {
            if let Err(e) = secrets::update_mode(&args) {
                eprintln!("Error refreshing secrets: {}", e);
            }
        }
        args::Mode::SecretRetrieve => {
            if let Err(e) = secrets::validate(&args) {
                eprintln!("Error retrieving secrets: {}", e);
            }
        }
        _ => {
            walk_dirs(&args);
        }
    }

    if args.verbose {
        println!("Done.");
    }

    Ok(())
}
