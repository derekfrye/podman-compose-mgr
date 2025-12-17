use crate::utils::build_logger::{BuildLogLevel, BuildLogger};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct RecordingLogger {
    messages: Arc<Mutex<Vec<(BuildLogLevel, String)>>>,
}

impl BuildLogger for RecordingLogger {
    fn log(&self, level: BuildLogLevel, message: &str) {
        self.messages
            .lock()
            .unwrap()
            .push((level, message.to_string()));
    }
}

impl RecordingLogger {
    pub fn logs(&self) -> Vec<(BuildLogLevel, String)> {
        self.messages.lock().unwrap().clone()
    }
}
