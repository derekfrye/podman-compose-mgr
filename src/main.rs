mod args;
mod image_build;
mod image_cmd;
mod podman;
mod image_restart;

use args::Args;



use regex::Regex;
use walkdir::WalkDir;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};



fn main() -> io::Result<()> {
    // Parse command-line arguments
    let args = args::args_checks();

    // if args.verbose {
    //     println!("Path: {}", args.path);
    //     println!("Mode: {:?}", args.mode);
    //     if let Some(secrets_file) = &args.secrets_file {
    //         println!("Secrets file: {}", secrets_file.display());
    //     }
    // }

    match args.mode {
        args::Mode::Rebuild => image_build::rebuild(&args),
        args::Mode::Secrets => secrets(&args),
        args::Mode::RestartSvcs => image_restart::restart_services(&args),
    }

    Ok(())
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
