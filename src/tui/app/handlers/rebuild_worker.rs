use crate::args::Args;
use crate::image_build::buildfile_build::build_dockerfile_image;
use crate::image_build::buildfile_types::{BuildChoice, BuildFile, WhatWereBuilding};
use crate::image_build::rebuild::RebuildManager;
use crate::interfaces::CommandHelper;
use crate::interfaces::DefaultCommandHelper;
use crate::interfaces::ReadInteractiveInputHelper;
use crate::read_interactive_input::{
    GrammarFragment, ReadValResult,
    format::{do_prompt_formatting, unroll_grammar_into_string},
};
use crate::tui::app::state::{Msg, OutputStream, RebuildJobSpec, RebuildResult, Services};
use crate::utils::{
    build_logger::{BuildLogger, TuiBuildLogger},
    error_utils, podman_utils,
};
use crossbeam_channel::Sender;
use std::error::Error;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use walkdir::{DirEntry, WalkDir};

pub fn spawn_rebuild_thread(specs: Vec<RebuildJobSpec>, services: &Services, start_idx: usize) {
    if specs.is_empty() {
        return;
    }

    let tx = services.tx.clone();
    let args = services.args.clone();

    std::thread::spawn(move || {
        let mut last_idx = start_idx;
        for (idx, spec) in specs.into_iter().enumerate() {
            let job_idx = start_idx + idx;
            let _ = tx.send(Msg::RebuildJobStarted { job_idx });
            match run_job(job_idx, &spec, &args, &tx) {
                Ok(()) => {
                    let _ = tx.send(Msg::RebuildJobFinished {
                        job_idx,
                        result: RebuildResult::Success,
                    });
                }
                Err(err) => {
                    let _ = tx.send(Msg::RebuildJobOutput {
                        job_idx,
                        chunk: err.clone(),
                        stream: OutputStream::Stderr,
                    });
                    let _ = tx.send(Msg::RebuildJobFinished {
                        job_idx,
                        result: RebuildResult::Failure(err),
                    });
                }
            }
            last_idx = job_idx;
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
    tx: &Sender<Msg>,
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

    if is_dockerfile_entry(&entry) {
        return rebuild_dockerfile(&cmd_helper, &build_logger, &entry, spec, args);
    }

    let mut manager = RebuildManager::new(&cmd_helper, &read_helper, &build_logger);

    manager.rebuild(&entry, args).map_err(|e| e.to_string())
}

fn is_dockerfile_entry(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_string_lossy()
        .starts_with("Dockerfile")
}

fn rebuild_dockerfile(
    cmd_helper: &TuiCommandHelper,
    logger: &TuiBuildLogger,
    entry: &DirEntry,
    spec: &RebuildJobSpec,
    args: &Args,
) -> Result<(), String> {
    let image = spec.image.trim();
    if image.is_empty() {
        return Err("Image name is required for Dockerfile rebuild".to_string());
    }

    let path = entry.path();
    let parent_dir = path
        .parent()
        .ok_or_else(|| format!("No parent directory for {}", path.display()))?
        .to_path_buf();

    logger.info(&format!(
        "Building image {} from {}",
        image,
        entry.path().display()
    ));

    let build_file = BuildFile {
        filetype: BuildChoice::Dockerfile,
        filepath: Some(path.to_path_buf()),
        parent_dir,
        link_target_dir: std::fs::read_link(path).ok(),
        base_image: Some(image.to_string()),
        custom_img_nm: Some(image.to_string()),
        build_args: args.build_args.clone(),
        no_cache: args.no_cache,
    };

    let build_spec = WhatWereBuilding {
        file: build_file,
        follow_link: false,
    };

    build_dockerfile_image(cmd_helper, &build_spec).map_err(|e| e.to_string())
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
