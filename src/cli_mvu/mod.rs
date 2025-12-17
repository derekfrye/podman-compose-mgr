mod model;
mod prompt_helpers;
mod update;

use crate::args::Args;
use crate::image_build::buildfile_types::WhatWereBuilding;
use crate::interfaces::DefaultCommandHelper;
use crate::utils::log_utils::Logger;
use crossbeam_channel as xchan;
use model::{Model, Msg, PromptItem, Services, State};

pub fn run_cli_loop(args: &Args, logger: &Logger) {
    let (tx, rx) = xchan::unbounded::<Msg>();
    let int_rx = prompt_helpers::spawn_interrupt_listener(tx.clone());
    let services = Services {
        args,
        logger,
        tx: tx.clone(),
    };

    let mut model = Model::new();
    let _ = tx.send(Msg::Init);

    loop {
        xchan::select! {
            recv(rx) -> msg => if let Ok(msg) = msg { update::update(&mut model, msg, &services); },
            recv(int_rx) -> _ => update::update(&mut model, Msg::Interrupt, &services),
        }
        if matches!(model.state, State::Done) {
            break;
        }
    }
}

/// Run discovery once and automatically build or pull each image.
pub fn run_one_shot(args: &Args, logger: &Logger) {
    prompt_helpers::log_one_shot_start(args, logger);

    let include = prompt_helpers::compile_regexes(args.include_path_patterns.clone());
    let exclude = prompt_helpers::compile_regexes(args.exclude_path_patterns.clone());
    let items = prompt_helpers::collect_prompt_items(&args.path, &include, &exclude);

    if items.is_empty() {
        logger.info("No docker-compose.yml or .container files were discovered.");
        return;
    }

    let build_arg_refs: Vec<&str> = args.build_args.iter().map(String::as_str).collect();

    for item in items {
        process_one_shot_item(args, logger, &build_arg_refs, &item);
    }
}

fn process_one_shot_item(args: &Args, logger: &Logger, build_arg_refs: &[&str], item: &PromptItem) {
    let entry = prompt_helpers::find_entry(&item.entry);
    let build_plan =
        prompt_helpers::select_build_plan(&entry, &item.image, build_arg_refs, args.no_cache);

    if args.one_shot.is_dry_run() {
        emit_dry_run_message(item, build_plan.as_ref());
        return;
    }

    if let Some(plan) = build_plan {
        logger.info(&format!(
            "Building image {} (container {}) via {}",
            item.image,
            item.container,
            prompt_helpers::describe_build_plan(&plan)
        ));
        let cmd_helper = DefaultCommandHelper;
        if let Err(err) =
            crate::image_build::buildfile_build::build_image_from_spec(&cmd_helper, &plan)
        {
            logger.warn(&format!("Failed to build image {}: {}", item.image, err));
        }
    } else {
        logger.info(&format!(
            "No Dockerfile or Makefile near {}. Pulling image {}.",
            item.entry.display(),
            item.image
        ));
        let cmd_helper = DefaultCommandHelper;
        if let Err(err) = crate::image_build::rebuild::pull_image(&cmd_helper, &item.image) {
            logger.warn(&format!("Failed to pull image {}: {}", item.image, err));
        }
    }
}

fn emit_dry_run_message(item: &PromptItem, plan: Option<&WhatWereBuilding>) {
    match plan {
        Some(plan) => println!(
            "[dry-run] {} (container {}) -> build via {}",
            item.image,
            item.container,
            prompt_helpers::describe_build_plan(plan)
        ),
        None => println!(
            "[dry-run] {} (container {}) -> pull (no Dockerfile/Makefile near {})",
            item.image,
            item.container,
            item.entry.display()
        ),
    }
}

pub(crate) use prompt_helpers::find_entry;
