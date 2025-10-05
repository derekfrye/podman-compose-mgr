use crate::Args;
use crate::app::AppCore;
use crate::ports::InterruptPort;
use crate::domain::{DiscoveredImage, ImageDetails};
use crate::utils::log_utils::Logger;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use std::{
    io,
    time::{Duration, Instant},
};

use super::ui;

pub struct App {
    pub title: String,
    pub should_quit: bool,
    pub state: UiState,
    pub rows: Vec<ItemRow>,
    pub selected: usize,
    pub spinner_idx: usize,
    pub view_mode: ViewMode,
    pub modal: Option<ModalState>,
    pub all_items: Vec<DiscoveredImage>,
    pub root_path: std::path::PathBuf,
    pub current_path: Vec<String>,
    pub app_core: Option<std::sync::Arc<AppCore>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiState {
    Scanning,
    Ready,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    ByContainer,
    ByImage,
    ByFolderThenImage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModalState {
    ViewPicker { selected_idx: usize },
}

#[derive(Clone, Debug)]
pub struct ItemRow {
    pub checked: bool,
    pub image: String,
    pub container: Option<String>,
    pub source_dir: std::path::PathBuf,
    pub expanded: bool,
    pub details: Vec<String>,
    pub is_dir: bool,
    pub dir_name: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Quit,
    MoveUp,
    MoveDown,
    ToggleCheck,
    ExpandOrEnter,
    CollapseOrBack,
    OpenViewPicker,
    ViewPickerUp,
    ViewPickerDown,
    ViewPickerAccept,
    ViewPickerCancel,
    Interrupt,
    Tick,
    ScanResults(Vec<DiscoveredImage>),
}

impl Default for App {
    fn default() -> Self {
        Self {
            title: "Podman Compose Manager".to_string(),
            should_quit: false,
            state: UiState::Scanning,
            rows: Vec::new(),
            selected: 0,
            spinner_idx: 0,
            view_mode: ViewMode::ByContainer,
            modal: None,
            all_items: Vec::new(),
            root_path: std::path::PathBuf::new(),
            current_path: Vec::new(),
            app_core: None,
        }
    }
}

impl App {
    pub fn new() -> Self { Self::default() }

    // Back-compat for tests: map key codes (no modifiers) to messages and update
    pub fn on_key(&mut self, key: KeyCode) {
        if let Some(msg) = map_keycode_to_msg(self, key) {
            update(self, msg);
        }
    }

    fn rebuild_rows_for_view(&mut self) {
        match self.view_mode {
            ViewMode::ByContainer => {
                // No change needed; current rows already reflect container view if produced from scan
                // But if we previously switched from image view, we need to regenerate from a cache.
                self.rows = self.build_rows_for_container_view();
                self.selected = 0;
            }
            ViewMode::ByImage => {
                use std::collections::HashSet;
                let mut seen: HashSet<String> = HashSet::new();
                let mut new_rows: Vec<ItemRow> = Vec::new();
                for d in &self.all_items {
                    if seen.insert(d.image.clone()) {
                        new_rows.push(ItemRow {
                            checked: false,
                            image: d.image.clone(),
                            container: None,
                            source_dir: d.source_dir.clone(),
                            expanded: false,
                            details: Vec::new(),
                            is_dir: false,
                            dir_name: None,
                        });
                    }
                }
                self.rows = new_rows;
                self.selected = 0;
            }
            ViewMode::ByFolderThenImage => {
                self.current_path.clear();
                self.rows = self.build_rows_for_folder_view();
                self.selected = 0;
            }
        }
    }

    fn build_rows_for_container_view(&self) -> Vec<ItemRow> {
        let mut rows: Vec<ItemRow> = Vec::new();
        for d in &self.all_items {
            rows.push(ItemRow {
                checked: false,
                image: d.image.clone(),
                container: d.container.clone(),
                source_dir: d.source_dir.clone(),
                expanded: false,
                details: Vec::new(),
                is_dir: false,
                dir_name: None,
            });
        }
        rows
    }

    fn build_rows_for_folder_view(&self) -> Vec<ItemRow> {
        use std::collections::BTreeSet;
        let mut subdirs: BTreeSet<String> = BTreeSet::new();
        let mut images: BTreeSet<String> = BTreeSet::new();
        for d in &self.all_items {
            // Only consider items under root_path
            if let Ok(rel) = d.source_dir.strip_prefix(&self.root_path) {
                let comps: Vec<String> = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .collect();
                // Filter to current path
                if comps.len() >= self.current_path.len()
                    && comps.iter().take(self.current_path.len()).eq(self.current_path.iter())
                {
                    let remainder = &comps[self.current_path.len()..];
                    if remainder.is_empty() {
                        images.insert(d.image.clone());
                    } else {
                        subdirs.insert(remainder[0].clone());
                    }
                }
            }
        }
        let mut rows: Vec<ItemRow> = Vec::new();
        for dir in subdirs.into_iter() {
            rows.push(ItemRow {
                checked: false,
                image: String::new(),
                container: None,
                source_dir: self.root_path.join(self.current_path.iter().collect::<std::path::PathBuf>()).join(&dir),
                expanded: false,
                details: Vec::new(),
                is_dir: true,
                dir_name: Some(dir),
            });
        }
        for img in images.into_iter() {
            rows.push(ItemRow {
                checked: false,
                image: img,
                container: None,
                source_dir: self.root_path.join(self.current_path.iter().collect::<std::path::PathBuf>()),
                expanded: false,
                details: Vec::new(),
                is_dir: false,
                dir_name: None,
            });
        }
        rows
    }
}

/// Run the terminal UI application
///
/// # Arguments
///
/// * `args` - Command line arguments
/// * `logger` - Logger instance
///
/// # Returns
///
/// * `io::Result<()>` - Success or error
///
/// # Errors
///
/// Returns an error if the terminal setup fails or if the application crashes.
pub fn run(args: &Args, logger: &Logger) -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();
    app.root_path = args.path.clone();
    let tick_rate = Duration::from_millis(250);

    // Wire ports/adapters and start background scan via app core
    let discovery = std::sync::Arc::new(crate::infra::discovery_adapter::FsDiscovery);
    let podman = std::sync::Arc::new(crate::infra::podman_adapter::PodmanCli);
    let app_core = std::sync::Arc::new(AppCore::new(discovery, podman));
    app.app_core = Some(app_core.clone());

    let (tx, rx) = std::sync::mpsc::channel::<Vec<DiscoveredImage>>();
    start_background_scan(args, app_core.clone(), tx);

    // Interrupt channel (production: real Ctrl+C)
    let interrupt_rx = Box::new(crate::infra::interrupt_adapter::CtrlcInterruptor::new()).subscribe();

    // Run the app and handle cleanup on exit or error
    let res = run_loop(&mut terminal, &mut app, tick_rate, args, logger, rx, interrupt_rx);

    // Always restore terminal state, even on error
    let cleanup_result = cleanup_terminal(&mut terminal);

    // Handle any errors
    if let Err(err) = res {
        logger.warn(&format!("Error in TUI: {err}"));
    }

    // If cleanup failed but the app was ok, return that error
    cleanup_result?;

    Ok(())
}

// Separate function for terminal cleanup to ensure it always happens
fn cleanup_terminal<B: Backend + std::io::Write>(terminal: &mut Terminal<B>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

pub fn run_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    tick_rate: Duration,
    args: &Args,
    logger: &Logger,
    rx: std::sync::mpsc::Receiver<Vec<DiscoveredImage>>,
    interrupt_rx: std::sync::mpsc::Receiver<()>,
) -> io::Result<()> {
    let mut last_tick = Instant::now();

    logger.debug("TUI is running");

    while !app.should_quit {
        // Check for interrupt
        match interrupt_rx.try_recv() {
            Ok(()) => update(app, Msg::Interrupt),
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {}
        }
        // Check for scan results
        match rx.try_recv() {
            Ok(discovered) => update(app, Msg::ScanResults(discovered)),
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                if app.state == UiState::Scanning { app.state = UiState::Ready; }
            }
        }

        terminal.draw(|f| ui::draw(f, app, args))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        match crossterm::event::poll(timeout) {
            Ok(true) => {
                if let Ok(Event::Key(key)) = event::read() {
                    if let Some(msg) = map_key_event_to_msg(app, key) {
                        update(app, msg);
                    }
                }
            }
            Ok(false) => {}
            Err(_) => {}
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
            update(app, Msg::Tick);
        }
    }

    Ok(())
}

pub(super) const SPINNER_FRAMES: &[&str] = &["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"];

fn start_background_scan(
    args: &Args,
    app_core: std::sync::Arc<AppCore>,
    tx: std::sync::mpsc::Sender<Vec<DiscoveredImage>>,
) {
    let path = args.path.clone();
    let exclude = args.exclude_path_patterns.clone();
    let include = args.include_path_patterns.clone();

    std::thread::spawn(move || {
        let discovered = app_core
            .scan_images(path, include, exclude)
            .unwrap_or_else(|_| Vec::new());
        let _ = tx.send(discovered);
    });
}

impl App {
    fn build_rows_for_view_mode(&self, mode: ViewMode) -> Vec<ItemRow> {
        let mut clone = self.clone_for_build();
        clone.view_mode = mode;
        match mode {
            ViewMode::ByContainer => clone.build_rows_for_container_view(),
            ViewMode::ByImage => {
                // reuse the ByImage logic
                let mut seen = std::collections::HashSet::new();
                let mut rows = Vec::new();
                for d in &clone.all_items {
                    if seen.insert(d.image.clone()) {
                        rows.push(ItemRow {
                            checked: false,
                            image: d.image.clone(),
                            container: None,
                            source_dir: d.source_dir.clone(),
                            expanded: false,
                            details: Vec::new(),
                            is_dir: false,
                            dir_name: None,
                        });
                    }
                }
                rows
            }
            ViewMode::ByFolderThenImage => clone.build_rows_for_folder_view(),
        }
    }

    fn clone_for_build(&self) -> App {
        App {
            title: self.title.clone(),
            should_quit: self.should_quit,
            state: self.state,
            rows: Vec::new(),
            selected: 0,
            spinner_idx: 0,
            view_mode: self.view_mode,
            modal: None,
            all_items: self.all_items.clone(),
            root_path: self.root_path.clone(),
            current_path: self.current_path.clone(),
            app_core: self.app_core.clone(),
        }
    }
}

fn compute_details(app: &App, image: &str, source_dir: &std::path::Path) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("Compose dir: {}", source_dir.display()));

    if let Some(core) = &app.app_core {
        match core.image_details(image, source_dir) {
            Ok(ImageDetails { created_time_ago, pulled_time_ago, has_dockerfile, has_makefile }) => {
                if let Some(s) = created_time_ago { lines.push(format!("Created: {s}")); }
                if let Some(s) = pulled_time_ago { lines.push(format!("Pulled: {s}")); }
                if has_dockerfile { lines.push("Found Dockerfile".into()); }
                if has_makefile { lines.push("Found Makefile".into()); }
            }
            Err(e) => lines.push(format!("error: {e}")),
        }
    }

    lines
}

pub fn update(app: &mut App, msg: Msg) {
    match msg {
        Msg::Quit => app.should_quit = true,
        Msg::Interrupt => { app.should_quit = true; },
        Msg::MoveUp => {
            if app.selected > 0 { app.selected -= 1; }
        }
        Msg::MoveDown => {
            if app.selected + 1 < app.rows.len() { app.selected += 1; }
        }
        Msg::ToggleCheck => {
            if let Some(row) = app.rows.get_mut(app.selected) {
                row.checked = !row.checked;
            }
        }
        Msg::ExpandOrEnter => {
            if app.view_mode == ViewMode::ByFolderThenImage {
                if let Some(row) = app.rows.get(app.selected)
                    && row.is_dir
                    && let Some(name) = &row.dir_name
                {
                    app.current_path.push(name.clone());
                    app.rows = app.build_rows_for_folder_view();
                    app.selected = 0;
                } else if app.rows.get(app.selected).is_some() {
                    let (image, source_dir, already_expanded) = {
                        let r = &app.rows[app.selected];
                        (r.image.clone(), r.source_dir.clone(), r.expanded)
                    };
                    if !already_expanded {
                        let details = compute_details(app, &image, &source_dir);
                        if let Some(rm) = app.rows.get_mut(app.selected) {
                            rm.details = details;
                            rm.expanded = true;
                        }
                    }
                }
            } else if app.rows.get(app.selected).is_some() {
                let (image, source_dir, already_expanded) = {
                    let r = &app.rows[app.selected];
                    (r.image.clone(), r.source_dir.clone(), r.expanded)
                };
                if !already_expanded {
                    let details = compute_details(app, &image, &source_dir);
                    if let Some(rm) = app.rows.get_mut(app.selected) {
                        rm.details = details;
                        rm.expanded = true;
                    }
                }
            }
        }
        Msg::CollapseOrBack => {
            if app.view_mode == ViewMode::ByFolderThenImage {
                if let Some(row) = app.rows.get_mut(app.selected)
                    && !row.is_dir
                    && row.expanded
                {
                    row.expanded = false;
                } else if !app.current_path.is_empty() && let Some(last_name) = app.current_path.pop() {
                    app.rows = app.build_rows_for_folder_view();
                    app.selected = app
                        .rows
                        .iter()
                        .position(|r| r.is_dir && r.dir_name.as_deref() == Some(&last_name))
                        .unwrap_or(0);
                }
            } else if let Some(row) = app.rows.get_mut(app.selected) {
                row.expanded = false;
            }
        }
        Msg::OpenViewPicker => {
            let default_idx = match app.view_mode { ViewMode::ByContainer => 0, ViewMode::ByImage => 1, ViewMode::ByFolderThenImage => 2 };
            app.modal = Some(ModalState::ViewPicker { selected_idx: default_idx });
        }
        Msg::ViewPickerUp => {
            if let Some(ModalState::ViewPicker { selected_idx }) = &mut app.modal {
                if *selected_idx > 0 { *selected_idx -= 1; }
            }
        }
        Msg::ViewPickerDown => {
            if let Some(ModalState::ViewPicker { selected_idx }) = &mut app.modal {
                if *selected_idx < 2 { *selected_idx += 1; }
            }
        }
        Msg::ViewPickerAccept => {
            if let Some(ModalState::ViewPicker { selected_idx }) = &mut app.modal {
                app.view_mode = match *selected_idx { 0 => ViewMode::ByContainer, 1 => ViewMode::ByImage, 2 => ViewMode::ByFolderThenImage, _ => ViewMode::ByContainer };
                app.rebuild_rows_for_view();
                app.modal = None;
            }
        }
        Msg::ViewPickerCancel => {
            app.modal = None;
        }
        Msg::Tick => {
            app.spinner_idx = (app.spinner_idx + 1) % SPINNER_FRAMES.len();
        }
        Msg::ScanResults(discovered) => {
            app.all_items = discovered;
            app.rows = match app.view_mode {
                ViewMode::ByContainer => app.build_rows_for_container_view(),
                ViewMode::ByImage => {
                    let mut tmp = App::new();
                    tmp.all_items = app.all_items.clone();
                    tmp.build_rows_for_view_mode(ViewMode::ByImage)
                }
                ViewMode::ByFolderThenImage => app.build_rows_for_folder_view(),
            };
            app.state = UiState::Ready;
            app.selected = 0;
        }
    }
}

fn map_key_event_to_msg(app: &App, ev: KeyEvent) -> Option<Msg> {
    if ev.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char('c') | KeyCode::Char('C') = ev.code {
            return Some(Msg::Interrupt);
        }
    }
    map_keycode_to_msg(app, ev.code)
}

fn map_keycode_to_msg(app: &App, key: KeyCode) -> Option<Msg> {
    // If modal is open, keys control the modal
    if let Some(ModalState::ViewPicker { .. }) = app.modal {
        return Some(match key {
            KeyCode::Esc => Msg::ViewPickerCancel,
            KeyCode::Up => Msg::ViewPickerUp,
            KeyCode::Down => Msg::ViewPickerDown,
            KeyCode::Enter => Msg::ViewPickerAccept,
            _ => return None,
        });
    }

    // Normal mode
    Some(match key {
        KeyCode::Char('q') => Msg::Quit,
        KeyCode::Up => Msg::MoveUp,
        KeyCode::Down => Msg::MoveDown,
        KeyCode::Char(' ') | KeyCode::Enter => Msg::ToggleCheck,
        KeyCode::Right => Msg::ExpandOrEnter,
        KeyCode::Left => Msg::CollapseOrBack,
        KeyCode::Char('v') => Msg::OpenViewPicker,
        _ => return None,
    })
}
