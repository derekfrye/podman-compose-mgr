use crate::args::Args;
use crate::image_build::container_file::parse_container_file;
use crate::image_build::rebuild::{Image, build_rebuild_grammars, read_yaml_file};
use crate::interfaces::DefaultCommandHelper;
use crate::ports::InterruptPort;
use crate::utils::log_utils::Logger;
use crossbeam_channel as xchan;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
enum State {
    Discovering,
    Ready,
    Done,
}

#[derive(Debug)]
struct Model {
    state: State,
    items: Vec<PromptItem>,
    idx: usize,
    processed: Vec<Image>,
}

impl Model {
    fn new() -> Self {
        Self {
            state: State::Discovering,
            items: Vec::new(),
            idx: 0,
            processed: Vec::new(),
        }
    }
}

#[derive(Debug)]
enum Msg {
    Init,
    Discovered(Vec<PromptItem>),
    PromptStart,
    PromptInput(String),
    ActionDone,
    Interrupt,
}

struct Services<'a> {
    args: &'a Args,
    logger: &'a Logger,
    tx: xchan::Sender<Msg>,
}

pub fn run_cli_loop(args: &Args, logger: &Logger) {
    let (tx, rx) = xchan::unbounded::<Msg>();
    let interrupt_std =
        Box::new(crate::infra::interrupt_adapter::CtrlcInterruptor::new()).subscribe();
    let (int_tx, int_rx) = xchan::bounded::<()>(0);
    std::thread::spawn(move || {
        let _ = interrupt_std.recv();
        let _ = int_tx.send(());
    });
    let services = Services {
        args,
        logger,
        tx: tx.clone(),
    };

    let mut model = Model::new();
    let _ = tx.send(Msg::Init);

    loop {
        xchan::select! {
            recv(rx) -> msg => if let Ok(msg) = msg { update(&mut model, msg, &services); },
            recv(int_rx) -> _ => update(&mut model, Msg::Interrupt, &services),
        }
        if matches!(model.state, State::Done) {
            break;
        }
    }
}

#[derive(Debug, Clone)]
struct PromptItem {
    entry: PathBuf,
    image: String,
    container: String,
}

fn spawn_discovery(
    tx: xchan::Sender<Msg>,
    root: PathBuf,
    include: Vec<String>,
    exclude: Vec<String>,
) {
    std::thread::spawn(move || {
        let inc = compile_regexes(include);
        let exc = compile_regexes(exclude);
        let items = collect_prompt_items(&root, &inc, &exc);
        let _ = tx.send(Msg::Discovered(items));
    });
}

fn compile_regexes(patterns: Vec<String>) -> Vec<regex::Regex> {
    patterns
        .into_iter()
        .filter_map(|p| regex::Regex::new(&p).ok())
        .collect()
}

fn collect_prompt_items(root: &Path, include: &[regex::Regex], exclude: &[regex::Regex]) -> Vec<PromptItem> {
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
            let Some(container) = mapping
                .get("container_name")
                .and_then(|v| v.as_str())
            else {
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

fn spawn_prompt(tx: xchan::Sender<Msg>, _args: &Args, item: PromptItem) {
    std::thread::spawn(move || {
        let cmd = DefaultCommandHelper;

        // Build DirEntry from the file path
        let entry = walkdir::WalkDir::new(&item.entry)
            .into_iter()
            .filter_map(Result::ok)
            .find(|e| e.path() == item.entry)
            .expect("entry must exist");

        // Build grammars and read a single input, then send as message
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
            // Treat empty as no-op; restart prompt
            let _ = tx.send(Msg::PromptStart);
        }
    });
}

fn update(model: &mut Model, msg: Msg, services: &Services) {
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
        let entry = find_entry(&item.entry);
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
    std::thread::spawn(move || {
        let entry = find_entry(&item.entry);
        let build_args_refs: Vec<&str> = build_args.iter().map(String::as_str).collect();
        let _ = crate::image_build::buildfile::start(&entry, &item.image, &build_args_refs);
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
    eprintln!("Invalid input. Please enter p/N/d/b/s/?: ");
    let _ = services.tx.send(Msg::PromptStart);
}

fn send_action_done(tx: &xchan::Sender<Msg>) {
    let _ = tx.send(Msg::ActionDone);
}

fn find_entry(path: &Path) -> walkdir::DirEntry {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .find(|entry| entry.path() == path)
        .expect("entry must exist")
}
