use crate::cmx as cmd;

use std::fs;
use walkdir::DirEntry;

pub fn build_image_from_dockerfile(dir: &DirEntry, image_name: &str) {
    let mut dockerfile = dir.path().to_path_buf().parent().unwrap().to_path_buf();
    dockerfile.push("Dockerfile");

    if !dockerfile.is_file()
        || !fs::metadata(&dockerfile).is_ok()
        || !fs::File::open(&dockerfile).is_ok()
    {
        eprintln!("No Dockerfile found at '{}'", dockerfile.display());
        std::process::exit(1);
    }

    let z = dockerfile.display().to_string();

    let mut x = vec![];
    x.push("build");
    x.push("-t");
    x.push(image_name);
    x.push("-f");
    x.push(&z);

    cmd::exec_cmd("podman", x);
}
