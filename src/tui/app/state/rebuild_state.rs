use super::super::search::SearchState;
use crate::domain::InferenceSource;
use std::collections::VecDeque;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct RebuildState {
    pub jobs: Vec<RebuildJob>,
    pub active_idx: usize,
    pub scroll_y: u16,
    pub scroll_x: u16,
    pub work_queue_selected: usize,
    pub finished: bool,
    pub auto_scroll: bool,
    pub viewport_height: u16,
    pub viewport_width: u16,
    pub output_limit: usize,
    pub search: Option<SearchState>,
}

#[derive(Clone, Debug)]
pub struct DockerfileRowExtra {
    pub source: InferenceSource,
    pub dockerfile_name: String,
    pub quadlet_basename: Option<String>,
    pub image_name: Option<String>,
    pub created_time_ago: Option<String>,
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DockerfileNameEntry {
    pub dockerfile_path: PathBuf,
    pub source_dir: PathBuf,
    pub dockerfile_name: String,
    pub image_name: String,
    pub cursor: usize,
}

impl RebuildState {
    #[must_use]
    pub fn new(jobs: Vec<RebuildJob>, output_limit: usize) -> Self {
        Self {
            jobs,
            active_idx: 0,
            scroll_y: 0,
            scroll_x: 0,
            work_queue_selected: 0,
            finished: false,
            auto_scroll: true,
            viewport_height: 0,
            viewport_width: 0,
            output_limit: output_limit.max(1),
            search: None,
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
    #[must_use]
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

    #[must_use]
    pub fn from_spec(spec: &RebuildJobSpec) -> Self {
        Self::new(
            spec.image.clone(),
            spec.container.clone(),
            spec.entry_path.clone(),
            spec.source_dir.clone(),
        )
    }

    pub fn push_output(&mut self, stream: OutputStream, chunk: String, limit: usize) {
        let max_lines = limit.max(1);
        push_with_limit(
            &mut self.output,
            RebuildOutputLine {
                stream,
                text: chunk,
            },
            max_lines,
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
