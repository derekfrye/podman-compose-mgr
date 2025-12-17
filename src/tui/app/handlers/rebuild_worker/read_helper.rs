use crate::interfaces::ReadInteractiveInputHelper;
use crate::read_interactive_input::{
    GrammarFragment, ReadValResult,
    format::{do_prompt_formatting, unroll_grammar_into_string},
};
use crate::tui::app::state::{Msg, OutputStream};
use crossbeam_channel::Sender;

pub struct NonInteractiveReadHelper {
    pub tx: Sender<Msg>,
    pub job_idx: usize,
}

impl NonInteractiveReadHelper {
    pub fn new(tx: Sender<Msg>, job_idx: usize) -> Self {
        Self { tx, job_idx }
    }
}

impl ReadInteractiveInputHelper for NonInteractiveReadHelper {
    fn read_val_from_cmd_line_and_proceed(
        &self,
        grammars: &mut [GrammarFragment],
        size: Option<usize>,
    ) -> ReadValResult {
        if let Some(width) = size {
            do_prompt_formatting(grammars, width);
        }
        let prompt = unroll_grammar_into_string(grammars, false, true);
        let _ = self.tx.send(Msg::RebuildJobOutput {
            job_idx: self.job_idx,
            chunk: prompt,
            stream: OutputStream::Stdout,
        });
        let _ = self.tx.send(Msg::RebuildJobOutput {
            job_idx: self.job_idx,
            chunk: "Auto-selecting 'b' (build)".to_string(),
            stream: OutputStream::Stdout,
        });
        ReadValResult {
            user_entered_val: Some("b".to_string()),
            was_interrupted: false,
        }
    }
}
