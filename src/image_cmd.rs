use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

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
