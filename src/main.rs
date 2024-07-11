// use clap::builder::Str;
use std::path::Path;
use clap::{Parser, ValueEnum};
use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use walkdir::WalkDir;

/// Struct to handle command-line arguments
#[derive(Parser)]
struct Args {
    /// Path to the directory to traverse
    #[arg(short = 'p', long, value_name = "PATH", default_value = ".")]
    path: String,
    /// Mode to run the program in
    #[arg(short = 'm', long, default_value = "Rebuild", value_parser = clap::value_parser!(Mode))]
    mode: Mode,
}

/// Enumeration of possible modes
#[derive(Clone, ValueEnum)]
enum Mode {
    Rebuild,
    Secrets,
}


fn main() -> io::Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    match args.mode {
        Mode::Rebuild => rebuild(&args),
        Mode::Secrets => secrets(&args),
    }

    Ok(())
}

fn rebuild(args: &Args){
    // Define the pattern to match image names
    let pattern = Regex::new(r"^\s*image:\s*(.*)").unwrap();
    // Update the pattern to match the required image format with tags
    // let djf_pattern = Regex::new(r"^djf/[\w]+(:[\w][\w.-]{0,127})?$").unwrap();
    let djf_pattern = Regex::new(r"^\s*djf/[\w\d:._-]+$").unwrap();

    for entry in WalkDir::new(&args.path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.file_name() == "docker-compose.yml" {
            if let Ok(file) = File::open(entry.path()) {
                for line in io::BufReader::new(file).lines() {
                    if let Ok(line) = line {
                        if let Some(captures) = pattern.captures(&line) {
                            let image = captures.get(1).unwrap().as_str().trim();
                            // Check if the image matches the djf pattern
                            if !djf_pattern.is_match(image) {
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
                                    // Pull the image using podman and stream the output
                                    println!("Pulling image: {}", image);
                                    let mut child = Command::new("podman")
                                        .arg("pull")
                                        .arg(image)
                                        .stdout(Stdio::piped())
                                        .spawn()
                                        .expect("Failed to execute podman pull");

                                    if let Some(stdout) = child.stdout.take() {
                                        let reader = BufReader::new(stdout);
                                        for line in reader.lines() {
                                            if let Ok(line) = line {
                                                println!("{}", line);
                                            }
                                        }
                                    }

                                    let _ = child.wait().expect("Command wasn't running");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}


fn secrets(args: &Args) {
    // Define the pattern to match secrets
    let pattern = Regex::new(r"^\s*-\s*([\w\d_]+)").unwrap();

    for entry in WalkDir::new(&args.path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.file_name() == "docker-compose.yml" {
            if let Ok(file) = File::open(entry.path()) {
                for line in io::BufReader::new(file).lines() {
                    if let Ok(line) = line {
                        if let Some(captures) = pattern.captures(&line) {
                            let secret = captures.get(1).unwrap().as_str().trim();
                            // Check if the secret exists
                            let secret_path = Path::new(&args.path).join("secrets").join(secret);
                            if !secret_path.exists() {
                                let z = entry.path().parent().unwrap_or(Path::new("/")).display();
                                let mut z_display = format!("{}", z);
                                let z_len = z_display.len();
                                if z_len > 25 {
                                    let start = &z_display[..5];
                                    let end = &z_display[z_len - 20..];
                                    z_display = format!("{}...{}", start, end);
                                }
                                print!("Create secret '{}' for {}? (y/N): ", secret, z_display);
                                io::stdout().flush().unwrap();
                                let mut input = String::new();
                                io::stdin().read_line(&mut input).unwrap();
                                let input = input.trim();
                                if input.eq_ignore_ascii_case("y") {
                                    // Create the secret using podman and stream the output
                                    println!("Creating secret: {}", secret);
                                    let mut child = Command::new("podman")
                                        .arg("secrets")
                                        .arg("create")
                                        .arg(secret)
                                        .arg(secret_path)
                                        .stdout(Stdio::piped())
                                        .spawn()
                                        .expect("Failed to execute podman secrets create");

                                    if let Some(stdout) = child.stdout.take() {
                                        let reader = BufReader::new(stdout);
                                        for line in reader.lines() {
                                            if let Ok(line) = line {
                                                println!("{}", line);
                                            }
                                        }
                                    }

                                    let _ = child.wait().expect("Command wasn't running");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

