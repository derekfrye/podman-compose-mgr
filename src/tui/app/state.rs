use super::keymap::map_keycode_to_msg;
use crate::Args;
use crate::app::AppCore;
use crate::domain::DiscoveredImage;
use crate::utils::log_utils::Logger;
use crossbeam_channel as xchan;
use std::collections::VecDeque;
use std::path::PathBuf;

pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Clone, Debug)]
pub struct ItemRow {
    pub checked: bool,
    pub image: String,
    pub container: Option<String>,
    pub source_dir: PathBuf,
    pub entry_path: Option<PathBuf>,
    pub expanded: bool,
    pub details: Vec<String>,
    pub is_dir: bool,
    pub dir_name: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiState {
    Scanning,
    Ready,
    Rebuilding,
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
    WorkQueue { selected_idx: usize },
}

#[derive(Clone, Debug)]
pub enum Msg {
    Init,
    Key(crossterm::event::KeyEvent),
    Quit,
    MoveUp,
    MoveDown,
    ToggleCheck,
    ToggleCheckAll,
    ExpandOrEnter,
    CollapseOrBack,
    OpenViewPicker,
    ViewPickerUp,
    ViewPickerDown,
    ViewPickerAccept,
    ViewPickerCancel,
    WorkQueueUp,
    WorkQueueDown,
    WorkQueueSelect,
    OpenWorkQueue,
    CloseModal,
    Interrupt,
    Tick,
    ScanResults(Vec<DiscoveredImage>),
    DetailsReady {
        row: usize,
        details: Vec<String>,
    },
    StartRebuild,
    RebuildSessionCreated {
        jobs: Vec<RebuildJobSpec>,
    },
    RebuildJobStarted {
        job_idx: usize,
    },
    RebuildJobOutput {
        job_idx: usize,
        chunk: String,
        stream: OutputStream,
    },
    RebuildJobFinished {
        job_idx: usize,
        result: RebuildResult,
    },
    RebuildAdvance,
    RebuildAborted(String),
    RebuildAllDone,
    ScrollOutputUp,
    ScrollOutputDown,
    ScrollOutputPageUp,
    ScrollOutputPageDown,
    ScrollOutputTop,
    ScrollOutputBottom,
    ScrollOutputLeft,
    ScrollOutputRight,
    ExitRebuild,
}

pub struct Services {
    pub core: std::sync::Arc<AppCore>,
    pub root: PathBuf,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub tx: xchan::Sender<Msg>,
    pub args: Args,
}

pub type LoopChans<'a> = crate::mvu::LoopChans<'a, Msg>;
pub type Env<'a> = crate::mvu::Env<'a, Args, Logger, Services>;

#[derive(Clone)]
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
    pub root_path: PathBuf,
    pub current_path: Vec<String>,
    pub rebuild: Option<RebuildState>,
    pub auto_rebuild_all: bool,
    pub auto_rebuild_triggered: bool,
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
            root_path: PathBuf::new(),
            current_path: Vec::new(),
            rebuild: None,
            auto_rebuild_all: false,
            auto_rebuild_triggered: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RebuildState {
    pub jobs: Vec<RebuildJob>,
    pub active_idx: usize,
    pub scroll_y: u16,
    pub scroll_x: u16,
    pub work_queue_selected: usize,
    pub finished: bool,
}

impl RebuildState {
    #[must_use]
    pub fn new(jobs: Vec<RebuildJob>) -> Self {
        Self {
            jobs,
            active_idx: 0,
            scroll_y: 0,
            scroll_x: 0,
            work_queue_selected: 0,
            finished: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RebuildJob {
    pub image: String,
    pub container: Option<String>,
    pub entry_path: PathBuf,
    pub source_dir: PathBuf,
    pub status: RebuildStatus,
    pub output: VecDeque<RebuildOutputLine>,
    pub error: Option<String>,
}

impl RebuildJob {
    pub fn new(
        image: String,
        container: Option<String>,
        entry_path: PathBuf,
        source_dir: PathBuf,
    ) -> Self {
        Self {
            image,
            container,
            entry_path,
            source_dir,
            status: RebuildStatus::Pending,
            output: VecDeque::new(),
            error: None,
        }
    }

    pub fn from_spec(spec: &RebuildJobSpec) -> Self {
        Self::new(
            spec.image.clone(),
            spec.container.clone(),
            spec.entry_path.clone(),
            spec.source_dir.clone(),
        )
    }

    pub fn push_output(&mut self, stream: OutputStream, chunk: String) {
        const MAX_LINES: usize = 4_096;
        push_with_limit(
            &mut self.output,
            RebuildOutputLine {
                stream,
                text: chunk,
            },
            MAX_LINES,
        );
    }
}

fn push_with_limit<T>(buf: &mut VecDeque<T>, line: T, limit: usize) {
    buf.push_back(line);
    if buf.len() > limit {
        let overflow = buf.len() - limit;
        for _ in 0..overflow {
            buf.pop_front();
        }
    }
}

#[derive(Clone, Debug)]
pub struct RebuildOutputLine {
    pub stream: OutputStream,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct RebuildJobSpec {
    pub image: String,
    pub container: Option<String>,
    pub entry_path: PathBuf,
    pub source_dir: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RebuildStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug)]
pub enum RebuildResult {
    Success,
    Failure(String),
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

impl App {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_root_path(&mut self, root: PathBuf) {
        self.root_path = root;
    }

    pub fn on_key(&mut self, key: crossterm::event::KeyCode) {
        if let Some(msg) = map_keycode_to_msg(self, key) {
            super::handlers::update_with_services(self, msg, None);
        }
    }
}
