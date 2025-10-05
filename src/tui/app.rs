use crate::Args;
use crate::utils::log_utils::Logger;
use crate::utils::podman_utils::image::{get_podman_image_upstream_create_time, get_podman_ondisk_modify_time};
use crate::utils::podman_utils::terminal::file_exists_and_readable;
use crate::tui::discover::scan_images;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
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
    pub all_items: Vec<crate::tui::discover::DiscoveredImage>,
    pub root_path: std::path::PathBuf,
    pub current_path: Vec<String>,
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
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_key(&mut self, key: KeyCode) {
        // If a modal is open, handle modal navigation
        if let Some(ModalState::ViewPicker { selected_idx }) = &mut self.modal {
            match key {
                KeyCode::Esc => {
                    self.modal = None;
                }
                KeyCode::Up => {
                    if *selected_idx > 0 {
                        *selected_idx -= 1;
                    }
                }
                KeyCode::Down => {
                    // 0 = ByContainer, 1 = ByImage, 2 = ByFolderThenImage
                    if *selected_idx < 2 {
                        *selected_idx += 1;
                    }
                }
                KeyCode::Enter => {
                    // Apply selection
                    self.view_mode = match *selected_idx {
                        0 => ViewMode::ByContainer,
                        1 => ViewMode::ByImage,
                        2 => ViewMode::ByFolderThenImage,
                        _ => ViewMode::ByContainer,
                    };
                    self.rebuild_rows_for_view();
                    self.modal = None;
                }
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Up => {
                if self.selected > 0 { self.selected -= 1; }
            }
            KeyCode::Down => {
                if self.selected + 1 < self.rows.len() { self.selected += 1; }
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let Some(row) = self.rows.get_mut(self.selected) {
                    row.checked = !row.checked;
                }
            }
            KeyCode::Right => {
                if self.view_mode == ViewMode::ByFolderThenImage {
                    if let Some(row) = self.rows.get(self.selected)
                        && row.is_dir
                        && let Some(name) = &row.dir_name
                    {
                        self.current_path.push(name.clone());
                        self.rows = self.build_rows_for_folder_view();
                        self.selected = 0;
                    } else if let Some(row) = self.rows.get_mut(self.selected) && !row.expanded {
                        // Expand details for image rows in folder view
                        row.details = compute_details(row);
                        row.expanded = true;
                    }
                } else if let Some(row) = self.rows.get_mut(self.selected) && !row.expanded {
                    row.details = compute_details(row);
                    row.expanded = true;
                }
            }
            KeyCode::Left => {
                if self.view_mode == ViewMode::ByFolderThenImage {
                    // First, if current selection is an expanded image row, collapse it
                    if let Some(row) = self.rows.get_mut(self.selected)
                        && !row.is_dir
                        && row.expanded
                    {
                        row.expanded = false;
                        return;
                    }
                    // Otherwise, navigate up to parent folder if possible
                    if !self.current_path.is_empty()
                        && let Some(last_name) = self.current_path.pop()
                    {
                        self.rows = self.build_rows_for_folder_view();
                        // Reselect the folder we just navigated out of in the parent view
                        self.selected = self
                            .rows
                            .iter()
                            .position(|r| r.is_dir && r.dir_name.as_deref() == Some(&last_name))
                            .unwrap_or(0);
                    }
                } else if let Some(row) = self.rows.get_mut(self.selected) {
                    row.expanded = false;
                }
            }
            KeyCode::Char('v') => {
                // Open view picker modal, default selection reflects current view
                let default_idx = match self.view_mode { ViewMode::ByContainer => 0, ViewMode::ByImage => 1, ViewMode::ByFolderThenImage => 2 };
                self.modal = Some(ModalState::ViewPicker { selected_idx: default_idx });
            }
            _ => {}
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

    // Start background scan
    let (tx, rx) = std::sync::mpsc::channel::<Vec<crate::tui::discover::DiscoveredImage>>();
    start_background_scan(args, tx);

    // Run the app and handle cleanup on exit or error
    let res = run_app(&mut terminal, &mut app, tick_rate, args, logger, rx);

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

fn run_app<B: Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    tick_rate: Duration,
    args: &Args,
    logger: &Logger,
    rx: std::sync::mpsc::Receiver<Vec<crate::tui::discover::DiscoveredImage>>,
) -> io::Result<()> {
    let mut last_tick = Instant::now();

    logger.debug("TUI is running");

    while !app.should_quit {
        // Check for scan results
        match rx.try_recv() {
            Ok(discovered) => {
                app.all_items = discovered;
                // Build rows for current view
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
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // Treat as no results
                if app.state == UiState::Scanning { app.state = UiState::Ready; }
            }
        }

        terminal.draw(|f| ui::draw(f, app, args))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? && let Event::Key(key) = event::read()? {
            app.on_key(key.code);
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
            // advance spinner
            app.spinner_idx = (app.spinner_idx + 1) % SPINNER_FRAMES.len();
        }
    }

    Ok(())
}

pub(super) const SPINNER_FRAMES: &[&str] = &["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"];

fn start_background_scan(args: &Args, tx: std::sync::mpsc::Sender<Vec<crate::tui::discover::DiscoveredImage>>) {
    let path = args.path.clone();
    let exclude = args.exclude_path_patterns.clone();
    let include = args.include_path_patterns.clone();
    let verbosity = args.verbose;
    let tmp = args.temp_file_path.clone();

    std::thread::spawn(move || {
        let args_c = Args {
            path,
            verbose: verbosity,
            exclude_path_patterns: exclude,
            include_path_patterns: include,
            build_args: vec![],
            temp_file_path: tmp,
            tui: true,
        };
        let logger = Logger::new(args_c.verbose);
        let discovered = scan_images(&args_c, &logger);
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
        }
    }
}

fn compute_details(row: &ItemRow) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("Compose dir: {}", row.source_dir.display()));

    match get_podman_image_upstream_create_time(&row.image) {
        Ok(dt) => lines.push(format!("Created: {}", crate::utils::podman_utils::datetime::format_time_ago(dt))),
        Err(e) => lines.push(format!("Created: error: {e}")),
    }
    match get_podman_ondisk_modify_time(&row.image) {
        Ok(dt) => lines.push(format!("Pulled: {}", crate::utils::podman_utils::datetime::format_time_ago(dt))),
        Err(e) => lines.push(format!("Pulled: error: {e}")),
    }

    let dockerfile = row.source_dir.join("Dockerfile");
    let makefile = row.source_dir.join("Makefile");
    lines.push(format!(
        "Dockerfile exists: {}",
        file_exists_and_readable(&dockerfile)
    ));
    lines.push(format!(
        "Makefile exists: {}",
        file_exists_and_readable(&makefile)
    ));

    lines
}
