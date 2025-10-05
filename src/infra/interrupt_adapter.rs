use std::sync::mpsc::{self, Receiver};

use crate::ports::InterruptPort;

pub struct CtrlcInterruptor {
    rx: Receiver<()>,
}

impl CtrlcInterruptor {
    /// Create a new interruptor wired to OS Ctrl+C.
    ///
    /// # Panics
    /// Panics if setting the Ctrl+C handler fails.
    #[must_use]
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let tx2 = tx.clone();
        ctrlc::set_handler(move || {
            let _ = tx2.send(());
        })
        .expect("Error setting Ctrl+C handler");
        Self { rx }
    }
}

impl Default for CtrlcInterruptor {
    fn default() -> Self {
        Self::new()
    }
}

impl InterruptPort for CtrlcInterruptor {
    fn subscribe(self: Box<Self>) -> Receiver<()> {
        self.rx
    }
}
