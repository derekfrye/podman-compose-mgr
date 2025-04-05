use podman_compose_mgr::secrets::debug;

fn main() {
    match debug::debug_azure_credentials() {
        Ok(_) => println!("Debug completed successfully"),
        Err(e) => eprintln!("Error during debugging: {}", e),
    }
}