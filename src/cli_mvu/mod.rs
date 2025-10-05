use crate::args::Args;
use crate::image_build::container_file::parse_container_file;
use crate::image_build::rebuild::{read_val_loop, read_yaml_file, Image, build_rebuild_grammars};
use crate::interfaces::DefaultCommandHelper;
use crate::utils::log_utils::Logger;
use std::path::PathBuf;
use crossbeam_channel as xchan;
use crate::ports::InterruptPort;

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
        Self { state: State::Discovering, items: Vec::new(), idx: 0, processed: Vec::new() }
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
    let interrupt_std = Box::new(crate::infra::interrupt_adapter::CtrlcInterruptor::new()).subscribe();
    let (int_tx, int_rx) = xchan::bounded::<()>(0);
    std::thread::spawn(move || { let _ = interrupt_std.recv(); let _ = int_tx.send(()); });
    let services = Services { args, logger, tx: tx.clone() };

    let mut model = Model::new();
    let _ = tx.send(Msg::Init);

    loop {
        xchan::select! {
            recv(rx) -> msg => { if let Ok(msg) = msg { update(&mut model, msg, &services); } },
            recv(int_rx) -> _ => { update(&mut model, Msg::Interrupt, &services); },
        }
        if matches!(model.state, State::Done) { break; }
    }
}

#[derive(Debug, Clone)]
struct PromptItem { entry: PathBuf, image: String, container: String }

fn spawn_discovery(tx: xchan::Sender<Msg>, root: PathBuf, include: Vec<String>, exclude: Vec<String>) {
    std::thread::spawn(move || {
        use regex::Regex;
        use walkdir::WalkDir;
        let mut inc: Vec<Regex> = Vec::new();
        let mut exc: Vec<Regex> = Vec::new();
        for p in include { if let Ok(r) = Regex::new(&p) { inc.push(r); } }
        for p in exclude { if let Ok(r) = Regex::new(&p) { exc.push(r); } }
        let mut items: Vec<PromptItem> = Vec::new();
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() { continue; }
            let Some(pstr) = entry.path().to_str() else { continue };
            if !exc.is_empty() && exc.iter().any(|r| r.is_match(pstr)) { continue; }
            if !inc.is_empty() && inc.iter().all(|r| !r.is_match(pstr)) { continue; }
            if entry.file_name() == "docker-compose.yml" {
                if let Ok(yaml) = read_yaml_file(pstr) {
                    if let Some(services) = yaml.get("services").and_then(|v| v.as_mapping()) {
                        for (_, svc_cfg) in services {
                            let Some(m) = svc_cfg.as_mapping() else { continue };
                            let Some(img) = m.get("image").and_then(|v| v.as_str()) else { continue };
                            let Some(container) = m.get("container_name").and_then(|v| v.as_str()) else { continue };
                            items.push(PromptItem { entry: entry.path().to_path_buf(), image: img.to_string(), container: container.to_string() });
                        }
                    }
                }
            } else if entry.path().extension().and_then(|s| s.to_str()) == Some("container") {
                if let Ok(info) = parse_container_file(entry.path()) {
                    let container = info.name.unwrap_or_else(|| entry.path().file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string());
                    items.push(PromptItem { entry: entry.path().to_path_buf(), image: info.image, container });
                }
            }
        }
        let _ = tx.send(Msg::Discovered(items));
    });
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
        Msg::Init => {
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
        Msg::Discovered(items) => {
            model.items = items;
            model.idx = 0;
            model.state = State::Ready;
            // Immediately prompt first file if any
            if let Some(item) = model.items.get(0).cloned() {
                spawn_prompt(services.tx.clone(), services.args, item);
            } else {
                model.state = State::Done;
            }
        }
        Msg::PromptStart => {
            if let Some(item) = model.items.get(model.idx).cloned() {
                spawn_prompt(services.tx.clone(), services.args, item);
            } else {
                model.state = State::Done;
            }
        }
        Msg::PromptInput(s) => {
            let choice = s.trim();
            if let Some(item) = model.items.get(model.idx).cloned() {
                match choice {
                    "p" => {
                        let tx = services.tx.clone();
                        let image = item.image.clone();
                        std::thread::spawn(move || {
                            let _ = crate::image_build::rebuild::pull_image(&DefaultCommandHelper, &image);
                            let _ = tx.send(Msg::ActionDone);
                        });
                    }
                    "N" => {
                        let _ = services.tx.send(Msg::ActionDone);
                    }
                    "d" => {
                        let tx = services.tx.clone();
                        std::thread::spawn(move || {
                            // Recreate entry
                            let entry = walkdir::WalkDir::new(&item.entry)
                                .into_iter().filter_map(Result::ok)
                                .find(|e| e.path() == item.entry).expect("entry must exist");
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
                    "?" => {
                        let tx = services.tx.clone();
                        std::thread::spawn(move || {
                            crate::image_build::ui::display_help();
                            let _ = tx.send(Msg::PromptStart);
                        });
                    }
                    "b" => {
                        let tx = services.tx.clone();
                        let build_args: Vec<String> = services.args.build_args.clone();
                        std::thread::spawn(move || {
                            // Recreate entry
                            let entry = walkdir::WalkDir::new(&item.entry)
                                .into_iter().filter_map(Result::ok)
                                .find(|e| e.path() == item.entry).expect("entry must exist");
                            let build_args_refs: Vec<&str> = build_args.iter().map(String::as_str).collect();
                            let _ = crate::image_build::buildfile::start(&entry, &item.image, &build_args_refs);
                            let _ = tx.send(Msg::ActionDone);
                        });
                    }
                    "s" => {
                        // Mark skip-all-by-name and advance
                        model.processed.push(Image { name: Some(item.image.clone()), container: Some(item.container.clone()), skipall_by_this_name: true });
                        let _ = services.tx.send(Msg::ActionDone);
                    }
                    _ => {
                        eprintln!("Invalid input. Please enter p/N/d/b/s/?: ");
                        let _ = services.tx.send(Msg::PromptStart);
                    }
                }
            } else {
                model.state = State::Done;
            }
        }
        Msg::ActionDone => {
            // Advance to next file
            model.idx += 1;
            if let Some(item) = model.items.get(model.idx).cloned() {
                // Skip duplicates based on processed list
                let skip = model.processed.iter().any(|i| {
                    i.name.as_deref() == Some(&item.image) && i.skipall_by_this_name
                        || (i.name.as_deref() == Some(&item.image)
                            && i.container.as_deref() == Some(&item.container))
                });
                if skip {
                    // pretend done; re-enqueue self to handle next
                    let _ = services.tx.send(Msg::ActionDone);
                } else {
                    // Mark processed for skip-by-name behavior going forward
                    model.processed.push(Image { name: Some(item.image.clone()), container: Some(item.container.clone()), skipall_by_this_name: true });
                    spawn_prompt(services.tx.clone(), services.args, item);
                }
            } else {
                model.state = State::Done;
            }
        }
        Msg::Interrupt => {
            model.state = State::Done;
        }
    }
}
