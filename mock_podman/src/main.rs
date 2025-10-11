use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

fn main() {
    if let Err(err) = run() {
        eprintln!("mock podman error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err("missing subcommand".into());
    }

    match args[1].as_str() {
        "pull" => handle_pull(&args[2..]),
        "build" => handle_build(&args[2..]),
        other => Err(format!("unsupported podman command: {other}").into()),
    }
}

fn handle_pull(args: &[String]) -> Result<(), Box<dyn Error>> {
    let image = args.last().cloned().unwrap_or_else(|| "unknown".to_string());
    println!("Trying to pull {image}");
    println!("Resolved {image}");
    io::stdout().flush()?;
    Ok(())
}

fn handle_build(args: &[String]) -> Result<(), Box<dyn Error>> {
    let dockerfile = locate_dockerfile(args)?;
    if dockerfile.ends_with("tests/test07/ddns/Dockerfile") {
        stream_build_output(DDNS_OUTPUT);
    } else if dockerfile.ends_with("tests/test07/rclone/Dockerfile") {
        stream_build_output(RCLONE_OUTPUT);
    } else {
        println!("STEP 1/1: FROM scratch");
        println!("--> mockimage");
        println!("COMMIT mock/image");
        println!("--> deadbeef");
        println!("Successfully tagged mock/image:latest");
        println!("deadbeef");
        io::stdout().flush()?;
    }
    Ok(())
}

fn locate_dockerfile(args: &[String]) -> Result<String, Box<dyn Error>> {
    let mut idx = 0;
    while idx < args.len() {
        if args[idx] == "-f" {
            return args
                .get(idx + 1)
                .cloned()
                .ok_or_else(|| "missing value for -f".into());
        }
        idx += 1;
    }

    Err("unable to resolve dockerfile path".into())
}

fn stream_build_output(script: &str) {
    for line in script.split('\n') {
        println!("{line}");
        let _ = io::stdout().flush();
        if line.starts_with("STEP ") {
            thread::sleep(Duration::from_millis(150));
        } else if line.starts_with("--> ") {
            thread::sleep(Duration::from_millis(60));
        }
    }
}

const DDNS_OUTPUT: &str = include_str!("../data/ddns_build.txt");
const RCLONE_OUTPUT: &str = include_str!("../data/rclone_build.txt");
