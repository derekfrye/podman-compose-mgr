use crate::interfaces::{CommandHelper, DefaultCommandHelper};
use crate::tui::app::state::{Msg, OutputStream};
use crate::utils::{error_utils, podman_utils};
use crossbeam_channel::Sender;
use std::error::Error;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

pub struct TuiCommandHelper {
    pub tx: Sender<Msg>,
    pub job_idx: usize,
    pub default: DefaultCommandHelper,
}

impl TuiCommandHelper {
    pub fn new(tx: Sender<Msg>, job_idx: usize) -> Self {
        Self {
            tx,
            job_idx,
            default: DefaultCommandHelper,
        }
    }

    pub fn send_line(&self, stream: OutputStream, line: String) {
        let _ = self.tx.send(Msg::RebuildJobOutput {
            job_idx: self.job_idx,
            chunk: line,
            stream,
        });
    }

    fn pipe_stream<R: std::io::Read + Send + 'static>(
        tx: Sender<Msg>,
        job_idx: usize,
        stream: OutputStream,
        pipe: Option<R>,
    ) -> Option<std::thread::JoinHandle<()>> {
        pipe.map(|pipe| {
            std::thread::spawn(move || {
                let reader = BufReader::new(pipe);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = tx.send(Msg::RebuildJobOutput {
                        job_idx,
                        chunk: line,
                        stream,
                    });
                }
            })
        })
    }
}

impl CommandHelper for TuiCommandHelper {
    fn exec_cmd(&self, cmd: &str, args: Vec<String>) -> Result<(), Box<dyn Error>> {
        if args.is_empty() {
            self.send_line(OutputStream::Stdout, format!("$ {cmd}"));
        } else {
            self.send_line(
                OutputStream::Stdout,
                format!("$ {} {}", cmd, args.join(" ")),
            );
        }

        let resolved_cmd = if cmd == "podman" {
            podman_utils::resolve_podman_binary()
        } else {
            std::ffi::OsString::from(cmd)
        };

        let mut child = Command::new(resolved_cmd)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout_handle = Self::pipe_stream(
            self.tx.clone(),
            self.job_idx,
            OutputStream::Stdout,
            child.stdout.take(),
        );
        let stderr_handle = Self::pipe_stream(
            self.tx.clone(),
            self.job_idx,
            OutputStream::Stderr,
            child.stderr.take(),
        );

        let status = child.wait()?;

        if let Some(handle) = stdout_handle {
            let _ = handle.join();
        }
        if let Some(handle) = stderr_handle {
            let _ = handle.join();
        }

        if !status.success() {
            return Err(error_utils::new_error(&format!(
                "Command '{cmd}' failed with status {status}"
            )));
        }

        Ok(())
    }

    fn pull_base_image(&self, dockerfile: &std::path::Path) -> Result<(), Box<dyn Error>> {
        self.send_line(
            OutputStream::Stdout,
            format!("Pulling base image for {}", dockerfile.display()),
        );
        self.default.pull_base_image(dockerfile)
    }

    fn get_terminal_display_width(&self, specify_size: Option<usize>) -> usize {
        self.default.get_terminal_display_width(specify_size)
    }

    fn file_exists_and_readable(&self, file: &std::path::Path) -> bool {
        self.default.file_exists_and_readable(file)
    }
}
