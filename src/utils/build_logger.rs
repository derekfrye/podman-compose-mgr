use crate::tui::app::{Msg, OutputStream};
use crate::utils::log_utils::Logger;
use crossbeam_channel::Sender;

/// Severity for build-related messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildLogLevel {
    Info,
    Warn,
    Error,
}

/// Interface for routing build feedback to the active frontend (CLI or TUI).
pub trait BuildLogger: Send + Sync {
    fn log(&self, level: BuildLogLevel, message: &str);

    fn info(&self, message: &str) {
        self.log(BuildLogLevel::Info, message);
    }

    fn warn(&self, message: &str) {
        self.log(BuildLogLevel::Warn, message);
    }

    fn error(&self, message: &str) {
        self.log(BuildLogLevel::Error, message);
    }
}

/// CLI implementation that proxies through the shared [`Logger`].
#[derive(Clone)]
pub struct CliBuildLogger {
    logger: Logger,
}

impl CliBuildLogger {
    #[must_use]
    pub fn new(logger: &Logger) -> Self {
        Self {
            logger: logger.clone(),
        }
    }
}

impl BuildLogger for CliBuildLogger {
    fn log(&self, level: BuildLogLevel, message: &str) {
        match level {
            BuildLogLevel::Info => self.logger.info(message),
            BuildLogLevel::Warn | BuildLogLevel::Error => self.logger.warn(message),
        }
    }
}

/// TUI implementation that forwards messages into the rebuild job stream.
pub struct TuiBuildLogger {
    tx: Sender<Msg>,
    job_idx: usize,
}

impl TuiBuildLogger {
    #[must_use]
    pub fn new(tx: Sender<Msg>, job_idx: usize) -> Self {
        Self { tx, job_idx }
    }

    fn emit(&self, stream: OutputStream, message: &str) {
        let _ = self.tx.send(Msg::RebuildJobOutput {
            job_idx: self.job_idx,
            chunk: message.to_string(),
            stream,
        });
    }
}

impl BuildLogger for TuiBuildLogger {
    fn log(&self, level: BuildLogLevel, message: &str) {
        match level {
            BuildLogLevel::Info => self.emit(OutputStream::Stdout, message),
            BuildLogLevel::Warn | BuildLogLevel::Error => self.emit(OutputStream::Stderr, message),
        }
    }
}
