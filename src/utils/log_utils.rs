//! Logging utilities for the application

/// Log levels for controlling verbosity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Normal execution, no verbose flag
    Normal = 0,
    /// Info level, one verbose flag (-v)
    Info = 1,
    /// Debug level, two verbose flags (-v -v)
    Debug = 2,
}

/// Logger for application messages
pub struct Logger {
    /// Current verbosity level
    verbosity: u8,
}

impl Logger {
    /// Create a new logger with the specified verbosity
    pub fn new(verbosity: u8) -> Self {
        Self { verbosity }
    }

    /// Log a message if the current verbosity level is at least the specified level
    pub fn log(&self, msg: &str, level: LogLevel) {
        if self.verbosity >= level as u8 {
            match level {
                LogLevel::Normal => println!("{}", msg),
                LogLevel::Info => println!("info: {}", msg),
                LogLevel::Debug => println!("dbg: {}", msg),
            }
        }
    }

    /// Log at normal level (always displayed)
    pub fn normal(&self, msg: &str) {
        self.log(msg, LogLevel::Normal);
    }

    /// Log at info level (verbose >= 1)
    pub fn info(&self, msg: &str) {
        self.log(msg, LogLevel::Info);
    }

    /// Log at debug level (verbose >= 2)
    pub fn debug(&self, msg: &str) {
        self.log(msg, LogLevel::Debug);
    }

    /// Get current verbosity level
    pub fn verbosity(&self) -> u8 {
        self.verbosity
    }
}

// Module-level functions for backward compatibility and convenience when a Logger instance isn't available

/// Log a message if the verbosity level is at least the specified level
///
/// # Arguments
///
/// * `msg` - The message to log
/// * `verbosity` - The current verbosity level (0 = normal, 1 = info, 2+ = debug)
/// * `level` - The minimum level required for this message to be logged
pub fn log(msg: &str, verbosity: u8, level: LogLevel) {
    if verbosity >= level as u8 {
        match level {
            LogLevel::Normal => println!("{}", msg),
            LogLevel::Info => println!("info: {}", msg),
            LogLevel::Debug => println!("dbg: {}", msg),
        }
    }
}

/// Log at info level (verbose >= 1)
///
/// # Arguments
///
/// * `msg` - The message to log
/// * `verbosity` - The current verbosity level
pub fn info(msg: &str, verbosity: u8) {
    log(msg, verbosity, LogLevel::Info)
}

/// Log at debug level (verbose >= 2)
///
/// # Arguments
///
/// * `msg` - The message to log
/// * `verbosity` - The current verbosity level
pub fn debug(msg: &str, verbosity: u8) {
    log(msg, verbosity, LogLevel::Debug)
}

/// Always log a message, regardless of verbosity level
///
/// # Arguments
///
/// * `msg` - The message to log
pub fn always(msg: &str) {
    log(msg, 0, LogLevel::Normal)
}
