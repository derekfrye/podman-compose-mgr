use podman_compose_mgr::{args, run_app};

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() -> io::Result<()> {
    // Set up a global Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Use ctrlc crate to handle Ctrl+C globally
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!("\nOperation cancelled by user");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl+C handler");

    // Parse command-line arguments
    let args = args::args_checks();
    if let Err(e) = args.validate() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    // Run the application logic
    if let Err(e) = run_app(args) {
        eprintln!("Application error: {e}");
        std::process::exit(1);
    }

    Ok(())
}
