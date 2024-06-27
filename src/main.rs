use clap::Parser;
use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead, Write};
use walkdir::WalkDir;
// use std::path::Path;
use std::process::Command;

/// Struct to handle command-line arguments
#[derive(Parser)]
struct Args {
    /// Path to the directory to traverse
    #[clap(long)]
    path: String,
}

fn main() -> io::Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Define the pattern to match image names
    let pattern = Regex::new(r"^\s*image:\s*(.*)").unwrap();
    let djf_pattern = Regex::new(r"^\s*djf/[\w\d:._-]+$").unwrap();
    // let djf_pattern = Regex::new(r"^djf/[\w]+(:[\w][\w.-]{0,127})?$").unwrap();

    // Traverse the specified directory and subdirectories
    for entry in WalkDir::new(&args.path).into_iter().filter_map(|e| e.ok()) {
        // Check if the file is docker-compose.yml
        if entry.file_type().is_file() && entry.file_name() == "docker-compose.yml" {
            // Open the file and read it line by line
            if let Ok(file) = File::open(entry.path()) {
                for line in io::BufReader::new(file).lines() {
                    if let Ok(line) = line {
                        // Check if the line matches the image pattern
                        if let Some(captures) = pattern.captures(&line) {
                            let image = captures.get(1).unwrap().as_str().trim();
                            // Check if the image matches the djf pattern
                            if !djf_pattern.is_match(image) {
                                // Ask user if they want to refresh the image
                                let z = entry
                                    .path()
                                    .parent()
                                    .unwrap_or(std::path::Path::new("/"))
                                    .display();
                                let mut z_display = format!("{}", z);
                                let z_len = z_display.len();
                                if z_len > 25 {
                                    let start = &z_display[..5];
                                    let end = &z_display[z_len - 20..];
                                    z_display = format!("{}...{}", start, end);
                                    // println!("{}", result);
                                }
                                print!("Refresh '{}' from {}? (y/N): ", image, z_display);
                                let mut input = String::new();
                                io::stdout().flush().unwrap();
                                io::stdin().read_line(&mut input).unwrap();
                                let input = input.trim();
                                if input.eq_ignore_ascii_case("y") {
                                    // Pull the image using podman
                                    println!("Pulling image: {}", image);
                                    let output = Command::new("podman")
                                        .arg("pull")
                                        .arg(image)
                                        .output()
                                        .expect("Failed to execute podman pull");
                                    io::stdout().write_all(&output.stdout).unwrap();
                                } else {
                                    println!("Skipping image: {}", image);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
