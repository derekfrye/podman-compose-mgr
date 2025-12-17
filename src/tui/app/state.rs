mod messages;
mod rebuild_state;

use super::keymap::map_keycode_to_msg;
use crate::Args;
use crate::app::AppCore;
use crate::domain::{DiscoveredImage, DockerfileInference};
use crate::utils::log_utils::Logger;
use crossbeam_channel as xchan;
use std::path::PathBuf;

pub use rebuild_state::{
    DockerfileNameEntry, DockerfileRowExtra, OutputStream, RebuildJob, RebuildJobSpec,
    RebuildResult, RebuildState, RebuildStatus,
};
#[cfg(test)]
pub use rebuild_state::RebuildOutputLine;

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
    pub dockerfile_extra: Option<DockerfileRowExtra>,
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
    ByDockerfile,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModalState {
    ViewPicker {
        selected_idx: usize,
    },
    WorkQueue {
        selected_idx: usize,
    },
    DockerfileNameEdit {
        entries: Vec<DockerfileNameEntry>,
        selected_idx: usize,
        error: Option<String>,
    },
    ExportLog {
        input: String,
        error: Option<String>,
    },
}

pub use messages::Msg;

pub struct Services {
    pub core: std::sync::Arc<AppCore>,
    pub root: PathBuf,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub tx: xchan::Sender<Msg>,
    pub args: Args,
    pub working_dir: PathBuf,
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
    pub dockerfile_items: Vec<DockerfileInference>,
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
            view_mode: ViewMode::ByImage,
            modal: None,
            all_items: Vec::new(),
            dockerfile_items: Vec::new(),
            root_path: PathBuf::new(),
            current_path: Vec::new(),
            rebuild: None,
            auto_rebuild_all: false,
            auto_rebuild_triggered: false,
        }
    }
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
