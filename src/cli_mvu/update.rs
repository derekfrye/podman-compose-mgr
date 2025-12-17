use crate::args::Args;
use crate::image_build::buildfile;
use crate::image_build::rebuild::{build_rebuild_grammars, Image};
use crate::interfaces::DefaultCommandHelper;
use crate::utils::build_logger::CliBuildLogger;
use crossbeam_channel as xchan;

use super::model::{Model, Msg, PromptItem, Services, State};
use super::prompt_helpers;

pub(super) fn update(model: &mut Model, msg: Msg, services: &Services) {
    match msg {
        Msg::Init => handle_init(services),
        Msg::Discovered(items) => handle_discovered(model, items, services),
        Msg::PromptStart => handle_prompt_start(model, services),
        Msg::PromptInput(s) => handle_prompt_input(model, s.trim(), services),
        Msg::ActionDone => handle_action_done(model, services),
        Msg::Interrupt => handle_interrupt(model),
    }
}

fn handle_init(services: &Services) {
    services.logger.info(&format!(
        "Rebuild images in path: {}",
        services.args.path.display()
    ));
    spawn_discovery(
        services.tx.clone(),
        services.args.path.clone(),
        services.args.include_path_patterns.clone(),
        services.args.exclude_path_patterns.clone(),
    );
}

fn handle_discovered(model: &mut Model, items: Vec<PromptItem>, services: &Services) {
    model.items = items;
    model.idx = 0;
    model.state = State::Ready;

    if let Some(item) = model.items.first().cloned() {
        spawn_prompt(services.tx.clone(), services.args, item);
    } else {
        model.state = State::Done;
    }
}

fn handle_prompt_start(model: &mut Model, services: &Services) {
    if let Some(item) = model.items.get(model.idx).cloned() {
        spawn_prompt(services.tx.clone(), services.args, item);
    } else {
        model.state = State::Done;
    }
}

fn handle_action_done(model: &mut Model, services: &Services) {
    model.idx += 1;
    if let Some(item) = model.items.get(model.idx).cloned() {
        if should_skip_item(model, &item) {
            let _ = services.tx.send(Msg::ActionDone);
            return;
        }

        model.processed.push(Image {
            name: Some(item.image.clone()),
            container: Some(item.container.clone()),
            skipall_by_this_name: true,
        });
        spawn_prompt(services.tx.clone(), services.args, item);
    } else {
        model.state = State::Done;
    }
}

fn handle_interrupt(model: &mut Model) {
    model.state = State::Done;
}

fn should_skip_item(model: &Model, item: &PromptItem) -> bool {
    model.processed.iter().any(|processed| {
        processed.name.as_deref() == Some(&item.image) && processed.skipall_by_this_name
            || (processed.name.as_deref() == Some(&item.image)
                && processed.container.as_deref() == Some(&item.container))
    })
}

fn handle_prompt_input(model: &mut Model, choice: &str, services: &Services) {
    let Some(item) = current_item(model) else {
        model.state = State::Done;
        return;
    };

    match choice {
        "p" => spawn_pull_image(services, item.image.clone()),
        "N" => send_action_done(&services.tx),
        "d" => spawn_display_details(services, item.clone()),
        "?" => spawn_help_prompt(services),
        "b" => spawn_build_prompt(services, item.clone()),
        "s" => mark_skip_and_advance(model, services, &item),
        _ => handle_invalid_choice(services),
    }
}

fn current_item(model: &Model) -> Option<PromptItem> {
    model.items.get(model.idx).cloned()
}

fn spawn_pull_image(services: &Services, image: String) {
    let tx = services.tx.clone();
    std::thread::spawn(move || {
        let _ = crate::image_build::rebuild::pull_image(&DefaultCommandHelper, &image);
        let _ = tx.send(Msg::ActionDone);
    });
}

fn spawn_display_details(services: &Services, item: PromptItem) {
    let tx = services.tx.clone();
    std::thread::spawn(move || {
        let entry = super::find_entry(&item.entry);
        crate::image_build::ui::display_image_info(
            &DefaultCommandHelper,
            &item.image,
            &item.container,
            &entry,
            &build_rebuild_grammars(&entry, &item.image, &item.container),
        );
        let _ = tx.send(Msg::PromptStart);
    });
}

fn spawn_help_prompt(services: &Services) {
    let tx = services.tx.clone();
    std::thread::spawn(move || {
        crate::image_build::ui::display_help();
        let _ = tx.send(Msg::PromptStart);
    });
}

fn spawn_build_prompt(services: &Services, item: PromptItem) {
    let tx = services.tx.clone();
    let build_args = services.args.build_args.clone();
    let build_logger = CliBuildLogger::new(services.logger);
    let logger = services.logger.clone();
    let no_cache = services.args.no_cache;
    std::thread::spawn(move || {
        let entry = super::find_entry(&item.entry);
        let build_args_refs: Vec<&str> = build_args.iter().map(String::as_str).collect();
        let cmd_helper = DefaultCommandHelper;
        if let Err(err) = buildfile::start(
            &cmd_helper,
            &entry,
            &item.image,
            &build_args_refs,
            &build_logger,
            no_cache,
        ) {
            logger.warn(&err.to_string());
        }
        let _ = tx.send(Msg::ActionDone);
    });
}

fn mark_skip_and_advance(model: &mut Model, services: &Services, item: &PromptItem) {
    model.processed.push(Image {
        name: Some(item.image.clone()),
        container: Some(item.container.clone()),
        skipall_by_this_name: true,
    });
    send_action_done(&services.tx);
}

fn handle_invalid_choice(services: &Services) {
    services
        .logger
        .warn("Invalid input. Please enter p/N/d/b/s/?: ");
    let _ = services.tx.send(Msg::PromptStart);
}

fn send_action_done(tx: &xchan::Sender<Msg>) {
    let _ = tx.send(Msg::ActionDone);
}

fn spawn_discovery(
    tx: xchan::Sender<Msg>,
    root: std::path::PathBuf,
    include: Vec<String>,
    exclude: Vec<String>,
) {
    std::thread::spawn(move || {
        let inc = prompt_helpers::compile_regexes(include);
        let exc = prompt_helpers::compile_regexes(exclude);
        let items = prompt_helpers::collect_prompt_items(&root, &inc, &exc);
        let _ = tx.send(Msg::Discovered(items));
    });
}

fn spawn_prompt(tx: xchan::Sender<Msg>, _args: &Args, item: PromptItem) {
    std::thread::spawn(move || {
        let cmd = DefaultCommandHelper;

        let entry = super::find_entry(&item.entry);

        let mut grammars = build_rebuild_grammars(&entry, &item.image, &item.container);
        let res = crate::read_interactive_input::read_val_from_cmd_line_and_proceed_with_deps(
            &mut grammars,
            &cmd,
            Box::new(crate::read_interactive_input::default_print),
            None,
            None,
        );
        if res.was_interrupted {
            let _ = tx.send(Msg::Interrupt);
        } else if let Some(s) = res.user_entered_val {
            let _ = tx.send(Msg::PromptInput(s));
        } else {
            let _ = tx.send(Msg::PromptStart);
        }
    });
}
