use podman_compose_mgr::{args, run_app};

fn main() {
    // Parse command-line arguments
    let args = args::args_checks();
    if let Err(e) = args.validate() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    // Run the application logic
    if let Err(e) = run_app(&args) {
        eprintln!("Application error: {e}");
        std::process::exit(1);
    }
}
