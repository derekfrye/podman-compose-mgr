use crate::args::Args;
use crate::image_build::buildfile_build::build_dockerfile_image;
use crate::image_build::buildfile_types::{BuildChoice, BuildFile, WhatWereBuilding};
use crate::image_build::rebuild::RebuildManager;
mod read_helper;
mod tui_command_helper;

use crate::tui::app::state::{Msg, OutputStream, RebuildJobSpec, RebuildResult, Services};
use crate::utils::build_logger::{BuildLogger, TuiBuildLogger};
use crossbeam_channel::Sender;
use read_helper::NonInteractiveReadHelper;
use tui_command_helper::TuiCommandHelper;
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
