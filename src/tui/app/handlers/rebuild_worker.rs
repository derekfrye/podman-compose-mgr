use crate::args::Args;
use crate::image_build::rebuild::RebuildManager;
use crate::interfaces::CommandHelper;
use crate::interfaces::DefaultCommandHelper;
use crate::interfaces::ReadInteractiveInputHelper;
use crate::read_interactive_input::{
    GrammarFragment, ReadValResult,
    format::{do_prompt_formatting, unroll_grammar_into_string},
};
use crate::tui::app::state::{Msg, OutputStream, RebuildJobSpec, RebuildResult, Services};
use crate::utils::{build_logger::TuiBuildLogger, error_utils, podman_utils};
use crossbeam_channel::Sender;
use std::error::Error;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use walkdir::WalkDir;

pub fn spawn_rebuild_thread(specs: Vec<RebuildJobSpec>, services: &Services) {
    if specs.is_empty() {
        return;
    }

    let tx = services.tx.clone();
    let args = services.args.clone();

    std::thread::spawn(move || {
        let mut last_idx = 0usize;
        for (idx, spec) in specs.into_iter().enumerate() {
            let _ = tx.send(Msg::RebuildJobStarted { job_idx: idx });
            match run_job(idx, &spec, &args, tx.clone()) {
                Ok(()) => {
                    let _ = tx.send(Msg::RebuildJobFinished {
                        job_idx: idx,
                        result: RebuildResult::Success,
                    });
                }
                Err(err) => {
                    let _ = tx.send(Msg::RebuildJobOutput {
                        job_idx: idx,
                        chunk: err.clone(),
                        stream: OutputStream::Stderr,
                    });
                    let _ = tx.send(Msg::RebuildJobFinished {
                        job_idx: idx,
                        result: RebuildResult::Failure(err),
                    });
                }
            }
            last_idx = idx;
        }

        let _ = tx.send(Msg::RebuildJobOutput {
            job_idx: last_idx,
            chunk: "Rebuild queue completed".to_string(),
            stream: OutputStream::Stdout,
        });
        let _ = tx.send(Msg::RebuildAllDone);
    });
}

fn run_job(
    job_idx: usize,
    spec: &RebuildJobSpec,
    args: &Args,
    tx: Sender<Msg>,
) -> Result<(), String> {
    let entry = WalkDir::new(&spec.entry_path)
        .max_depth(0)
        .into_iter()
        .next()
        .ok_or_else(|| format!("Unable to access {}", spec.entry_path.display()))
        .and_then(|res| res.map_err(|e| e.to_string()))?;

    let cmd_helper = TuiCommandHelper::new(tx.clone(), job_idx);
    let read_helper = NonInteractiveReadHelper::new(tx.clone(), job_idx);
    let build_logger = TuiBuildLogger::new(tx.clone(), job_idx);
    let mut manager = RebuildManager::new(&cmd_helper, &read_helper, &build_logger);

    manager.rebuild(&entry, args).map_err(|e| e.to_string())
}

struct TuiCommandHelper {
    tx: Sender<Msg>,
    job_idx: usize,
    default: DefaultCommandHelper,
}

impl TuiCommandHelper {
    fn new(tx: Sender<Msg>, job_idx: usize) -> Self {
        Self {
            tx,
            job_idx,
            default: DefaultCommandHelper,
        }
    }

    fn send_line(&self, stream: OutputStream, line: String) {
        let _ = self.tx.send(Msg::RebuildJobOutput {
            job_idx: self.job_idx,
            chunk: line,
            stream,
        });
    }
}

impl CommandHelper for TuiCommandHelper {
    fn exec_cmd(&self, cmd: &str, args: Vec<String>) -> Result<(), Box<dyn Error>> {
        if !args.is_empty() {
            self.send_line(
                OutputStream::Stdout,
                format!("$ {} {}", cmd, args.join(" ")),
            );
        } else {
            self.send_line(OutputStream::Stdout, format!("$ {cmd}"));
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

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let tx_out = self.tx.clone();
        let tx_err = self.tx.clone();
        let job_idx = self.job_idx;

        let stdout_handle = stdout.map(|pipe| {
            std::thread::spawn(move || {
                let reader = BufReader::new(pipe);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = tx_out.send(Msg::RebuildJobOutput {
                        job_idx,
                        chunk: line,
                        stream: OutputStream::Stdout,
                    });
                }
            })
        });

        let stderr_handle = stderr.map(|pipe| {
            std::thread::spawn(move || {
                let reader = BufReader::new(pipe);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = tx_err.send(Msg::RebuildJobOutput {
                        job_idx,
                        chunk: line,
                        stream: OutputStream::Stderr,
                    });
                }
            })
        });

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

struct NonInteractiveReadHelper {
    tx: Sender<Msg>,
    job_idx: usize,
}

impl NonInteractiveReadHelper {
    fn new(tx: Sender<Msg>, job_idx: usize) -> Self {
        Self { tx, job_idx }
    }
}

impl ReadInteractiveInputHelper for NonInteractiveReadHelper {
    fn read_val_from_cmd_line_and_proceed(
        &self,
        grammars: &mut [GrammarFragment],
        size: Option<usize>,
    ) -> ReadValResult {
        if let Some(width) = size {
            do_prompt_formatting(grammars, width);
        }
        let prompt = unroll_grammar_into_string(grammars, false, true);
        let _ = self.tx.send(Msg::RebuildJobOutput {
            job_idx: self.job_idx,
            chunk: prompt,
            stream: OutputStream::Stdout,
        });
        let _ = self.tx.send(Msg::RebuildJobOutput {
            job_idx: self.job_idx,
            chunk: "Auto-selecting 'b' (build)".to_string(),
            stream: OutputStream::Stdout,
        });
        ReadValResult {
            user_entered_val: Some("b".to_string()),
            was_interrupted: false,
        }
    }
}
