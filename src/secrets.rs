use crate::args::Args;

use regex::Regex;
// use std::fs::File;
// use std::io::{self, BufRead, BufReader, Write};
use std::io::{BufRead, BufReader};
// use std::path::Path;
// use std::process::{Command, Stdio};
use walkdir::DirEntry;

pub fn secrets(args: &Args, entry: &DirEntry) {
    // Define the pattern to match secrets
    if args.verbose {
        println!("Checking for secrets in path: {}", args.path.display());
    }
    let _pattern = Regex::new(r"^\s*-\s*([\w\d_]+)").unwrap();

    if let Ok(file) = std::fs::File::open(entry.path()) {
        let reader = BufReader::new(file);
        for _line in reader.lines() {
            // Rest of the code
        }
    }
    //     if let Ok(line) = line {
    //         if let Some(captures) = pattern.captures(&line) {
    //             let secret = captures.get(1).unwrap().as_str().trim();
    //             // Check if the secret exists
    //             let secret_path = Path::new(&args.path).join("secrets").join(secret);
    //             if !secret_path.exists() {
    //                 let z = entry.path().parent().unwrap_or(Path::new("/")).display();
    //                 let mut z_display = format!("{}", z);
    //                 let z_len = z_display.len();
    //                 if z_len > 25 {
    //                     let start = &z_display[..5];
    //                     let end = &z_display[z_len - 20..];
    //                     z_display = format!("{}...{}", start, end);
    //                 }
    //                 print!("Create secret '{}' for {}? (y/N): ", secret, z_display);
    //                 io::stdout().flush().unwrap();
    //                 let mut input = String::new();
    //                 io::stdin().read_line(&mut input).unwrap();
    //                 let input = input.trim();
    //                 if input.eq_ignore_ascii_case("y") {
    //                     // Create the secret using podman and stream the output
    //                     println!("Creating secret: {}", secret);
    //                     let mut child = Command::new("podman")
    //                         .arg("secrets")
    //                         .arg("create")
    //                         .arg(secret)
    //                         .arg(secret_path)
    //                         .stdout(Stdio::piped())
    //                         .spawn()
    //                         .expect("Failed to execute podman secrets create");

    //                     if let Some(stdout) = child.stdout.take() {
    //                         let reader = BufReader::new(stdout);
    //                         for line in reader.lines() {
    //                             if let Ok(line) = line {
    //                                 println!("{}", line);
    //                             }
    //                         }
    //                     }

    //                     let _ = child.wait().expect("Command wasn't running");
    //                 }
    //             }
    //         }
    //     }
    // }
}
