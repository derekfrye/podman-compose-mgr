use std::process::{Command, ExitStatus};

fn main() {
    // Get number of CPU cores
    let cpu_count = num_cpus::get();
    
    // Define the command to run based on CPU count
    let status = if cpu_count <= 8 {
        // For 8 or fewer CPUs, limit concurrency
        println!("Running tests with limited concurrency (CPU count: {})...", cpu_count);
        run_command("cargo", &["test", "-j", "3", "--", "--test-threads=3"])
    } else {
        // For more than 8 CPUs, use full concurrency
        println!("Running tests with full concurrency (CPU count: {})...", cpu_count);
        run_command("cargo", &["test"])
    };
    
    // Exit with the same status code
    std::process::exit(status.code().unwrap_or(1));
}

fn run_command(cmd: &str, args: &[&str]) -> ExitStatus {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .expect("Failed to execute command");
        
    status
}