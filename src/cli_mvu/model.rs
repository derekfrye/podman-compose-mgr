use crate::args::Args;
use crate::image_build::rebuild::Image;
use crate::utils::log_utils::Logger;
use crossbeam_channel as xchan;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(super) enum State {
    Discovering,
    Ready,
    Done,
}

#[derive(Debug)]
pub(super) struct Model {
    pub(super) state: State,
    pub(super) items: Vec<PromptItem>,
    pub(super) idx: usize,
    pub(super) processed: Vec<Image>,
}

impl Model {
    pub(super) fn new() -> Self {
        Self {
            state: State::Discovering,
            items: Vec::new(),
            idx: 0,
            processed: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub(super) enum Msg {
    Init,
    Discovered(Vec<PromptItem>),
    PromptStart,
    PromptInput(String),
    ActionDone,
    Interrupt,
}

#[derive(Debug, Clone)]
pub(super) struct PromptItem {
    pub(super) entry: PathBuf,
    pub(super) image: String,
    pub(super) container: String,
}

pub(super) struct Services<'a> {
    pub(super) args: &'a Args,
    pub(super) logger: &'a Logger,
    pub(super) tx: xchan::Sender<Msg>,
}
