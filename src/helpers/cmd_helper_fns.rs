use dockerfile_parser::Dockerfile;
use std::io::{ BufRead, BufReader, Read };
use std::path::Path;
use std::process::{ Command, Stdio };
use terminal_size::{ self, Width };

/// Parse Dockerfile and pull base image
pub fn pull_base_image(dockerfile: &std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(dockerfile).unwrap();
    let mut reader = BufReader::new(file);

    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    let dockerfile = Dockerfile::parse(&content)?;

    let mut x = vec![];
    x.push("pull");
    let mut img_nm = vec![];
    let from_img = dockerfile.instructions;
    for i in from_img {
        if let dockerfile_parser::Instruction::From(image, ..) = i {
            img_nm.push(image.image.clone().to_string());
        }
    }
    x.push(&img_nm[0]);

    exec_cmd("podman", x);

    Ok(())
}

/// exists(), is_file() traversing links, and metadata.is_ok() traversing links
pub fn file_exists_and_readable(file: &Path) -> bool {
    let z = file.try_exists();
    match z {
        Ok(true) => file.is_file() && file.metadata().is_ok(),
        _ => false,
    }
}

pub fn exec_cmd(cmd: &str, args: &[&str]) {
    let mut cmd = Command::new(cmd);

    cmd.args(args);

    let mut x = cmd.stdout(Stdio::piped()).spawn().expect("Failed to execute command");

    if let Some(stdout) = x.stdout.take() {
        let reader = BufReader::new(stdout);

        reader
            .lines()
            .flatten()
            .for_each(|line| {
                println!("{}", line);
            });
    }

    let _ = x.wait().expect("Command wasn't running");
}

pub fn get_terminal_display_width() -> usize {
    let size = terminal_size::terminal_size();
    if let Some((Width(w), _)) = size {
        w as usize
    } else {
        80
    }
}
