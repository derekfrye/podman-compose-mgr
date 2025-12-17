use crate::image_build::buildfile_helpers;
use crate::image_build::buildfile_types::{BuildChoice, WhatWereBuilding};
use crate::image_build::container_file::parse_container_file;
use crate::image_build::rebuild::read_yaml_file;
use crate::ports::InterruptPort;
use crate::utils::log_utils::Logger;
use crossbeam_channel as xchan;
use std::path::Path;

use super::model::{Msg, PromptItem};

pub(super) fn compile_regexes(patterns: Vec<String>) -> Vec<regex::Regex> {
    patterns
        .into_iter()
        .filter_map(|p| regex::Regex::new(&p).ok())
        .collect()
}

pub(super) fn collect_prompt_items(
    root: &Path,
    include: &[regex::Regex],
    exclude: &[regex::Regex],
) -> Vec<PromptItem> {
    use walkdir::WalkDir;

    let mut items = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(path_str) = entry.path().to_str() else {
            continue;
        };

        if path_filtered(path_str, include, exclude) {
            continue;
        }

        if entry.file_name() == "docker-compose.yml" {
            push_compose_items(&entry, path_str, &mut items);
        } else {
            push_container_item(&entry, &mut items);
        }
    }

    items
}

fn path_filtered(path: &str, include: &[regex::Regex], exclude: &[regex::Regex]) -> bool {
    (!exclude.is_empty() && exclude.iter().any(|r| r.is_match(path)))
        || (!include.is_empty() && include.iter().all(|r| !r.is_match(path)))
}

fn push_compose_items(entry: &walkdir::DirEntry, path_str: &str, items: &mut Vec<PromptItem>) {
    if let Ok(yaml) = read_yaml_file(path_str)
        && let Some(services) = yaml.get("services").and_then(|v| v.as_mapping())
    {
        for svc_cfg in services.values() {
            let Some(mapping) = svc_cfg.as_mapping() else {
                continue;
            };
            let Some(image) = mapping.get("image").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(container) = mapping.get("container_name").and_then(|v| v.as_str()) else {
                continue;
            };

            items.push(PromptItem {
                entry: entry.path().to_path_buf(),
                image: image.to_string(),
                container: container.to_string(),
            });
        }
    }
}

fn push_container_item(entry: &walkdir::DirEntry, items: &mut Vec<PromptItem>) {
    if entry.path().extension().and_then(|s| s.to_str()) != Some("container") {
        return;
    }

    if let Ok(info) = parse_container_file(entry.path()) {
        let container = info.name.unwrap_or_else(|| {
            entry
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string()
        });

        items.push(PromptItem {
            entry: entry.path().to_path_buf(),
            image: info.image,
            container,
        });
    }
}

pub(super) fn select_build_plan(
    dir: &walkdir::DirEntry,
    image: &str,
    build_args: &[&str],
    no_cache: bool,
) -> Option<WhatWereBuilding> {
    let buildfiles = buildfile_helpers::find_buildfile(dir, image, build_args, no_cache)?;
    let chosen = buildfiles
        .into_iter()
        .find(|file| file.filepath.is_some())?;

    Some(WhatWereBuilding {
        file: chosen,
        follow_link: false,
    })
}

pub(super) fn describe_build_plan(plan: &WhatWereBuilding) -> String {
    match plan.file.filetype {
        BuildChoice::Dockerfile => plan.file.filepath.as_ref().map_or_else(
            || "Dockerfile".to_string(),
            |path| format!("Dockerfile at {}", path.display()),
        ),
        BuildChoice::Makefile => {
            let dir = if plan.follow_link {
                plan.file
                    .link_target_dir
                    .as_ref()
                    .and_then(|link| link.parent())
                    .unwrap_or(&plan.file.parent_dir)
            } else {
                &plan.file.parent_dir
            };
            format!("Makefile (make -C {})", dir.display())
        }
    }
}

pub(super) fn spawn_interrupt_listener(_tx: xchan::Sender<Msg>) -> xchan::Receiver<()> {
    let interrupt_std =
        Box::new(crate::infra::interrupt_adapter::CtrlcInterruptor::new()).subscribe();
    let (int_tx, int_rx) = xchan::bounded::<()>(0);
    std::thread::spawn(move || {
        let _ = interrupt_std.recv();
        let _ = int_tx.send(());
    });
    int_rx
}

pub(super) fn log_one_shot_start(args: &crate::args::Args, logger: &Logger) {
    logger.info(&format!(
        "One-shot processing images under {}",
        args.path.display()
    ));
}

pub(crate) fn find_entry(path: &Path) -> walkdir::DirEntry {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .find(|entry| entry.path() == path)
        .expect("entry must exist")
}
