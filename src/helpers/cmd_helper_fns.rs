use dockerfile_parser::Dockerfile;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};

pub fn pull_base_image(dockerfile: &std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(dockerfile).unwrap();
    let mut reader = BufReader::new(file);

    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    let dockerfile = Dockerfile::parse(&content)?;

    let mut x = vec![];
    x.push("pull");
    let mut img_nm = vec![];
    let fromimg = dockerfile.instructions;
    for i in fromimg {
        match i {
            dockerfile_parser::Instruction::From(image, ..) => {
                img_nm.push(image.image.clone().to_string());
            }
            _ => {}
        }
    }
    x.push(&img_nm[0]);

    exec_cmd("podman", x);

    Ok(())
}

pub fn dockerfile_exists_and_readable(dockerfile: &std::path::PathBuf) -> bool {
    dockerfile.exists() && dockerfile.is_file() && dockerfile.metadata().is_ok()
}

pub fn exec_cmd(cmd: &str, args: Vec<&str>) {
    let mut cmd = Command::new(cmd);

    cmd.args(args);

    let mut x = cmd
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    if let Some(stdout) = x.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                println!("{}", line);
            }
        }
    }

    let _ = x.wait().expect("Command wasn't running");
}

pub fn get_terminal_display_width() -> usize {
    let (width, _) = term_size::dimensions().unwrap_or((80, 24));
    width
}