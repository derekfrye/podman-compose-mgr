use std::sync::mpsc::{self, Receiver};

use crate::ports::InterruptPort;

pub struct CtrlcInterruptor {
    rx: Receiver<()>,
}

impl CtrlcInterruptor {
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

impl InterruptPort for CtrlcInterruptor {
    fn subscribe(self: Box<Self>) -> Receiver<()> {
        self.rx
    }
}

