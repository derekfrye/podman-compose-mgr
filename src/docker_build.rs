use walkdir::DirEntry;
use std::fs;
use std::process::Command;

pub fn build_image_from_dockerfile(dir: &DirEntry, image_name: &str) {

    let mut dockerfile = dir.path().to_path_buf();
    dockerfile.push("Dockerfile");

    if !dockerfile.is_file() || !fs::metadata(&dockerfile).is_ok() || !fs::File::open(&dockerfile).is_ok() {
        eprintln!("No Dockerfile found at '{}'", dockerfile.display());
        std::process::exit(1);
    }

    let mut cmd = Command::new("podman");
    cmd.arg("build");
    cmd.arg("-t");
    cmd.arg(image_name);
    cmd.arg(dockerfile);

    let status = cmd.status().expect("Failed to execute podman build");
    if !status.success() {
        eprintln!("Failed to build podman image");
        std::process::exit(1);
    }
    
}