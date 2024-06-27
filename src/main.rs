use walkdir::WalkDir;
use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead};
// use std::path::Path;
use std::process::Command;

fn main() -> io::Result<()> {
    // Define the pattern to match image names
    let pattern = Regex::new(r"^\s*image:\s*(.*)").unwrap();
    let djf_pattern = Regex::new(r"^djf/[\w]+$").unwrap();

    // Traverse the current directory and subdirectories
    for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
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
                                // Pull the image using podman
                                println!("Pulling image: {}", image);
                                let _output = Command::new("podman")
                                    .arg("pull")
                                    .arg(image)
                                    .output()
                                    .expect("Failed to execute podman pull");
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
