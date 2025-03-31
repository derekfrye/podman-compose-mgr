use podman_compose_mgr::{
    args::{self},
    compose_finder::walk_dirs,
};
use podman_compose_mgr::secrets::azure;
use podman_compose_mgr::secrets::validation;

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
            if let Err(e) = azure::update_mode(&args) {
                eprintln!("Error refreshing secrets: {}", e);
            }
        }
        args::Mode::SecretRetrieve => {
            if let Err(e) = validation::validate(&args) {
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
