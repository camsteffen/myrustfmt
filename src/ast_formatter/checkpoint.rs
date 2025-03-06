use std::rc::Rc;
use crate::ast_formatter::AstFormatter;
use crate::constraints::CheckpointCounter;
use crate::source_formatter::{SourceFormatterCheckpoint, SourceFormatterLookahead};

/// A checkpoint that can be cheaply restored, like an "undo".
/// All checkpoints must be closed *before* the final formatting strategy.
#[must_use]
pub struct Checkpoint {
    counter: Rc<CheckpointCounter>,
    sf_checkpoint: SourceFormatterCheckpoint,
}

impl Drop for Checkpoint {
    fn drop(&mut self) {
        self.counter.decrement();
    }
}

impl AstFormatter {
    pub fn open_checkpoint(&self) -> Checkpoint {
        let counter = Rc::clone(self.checkpoint_counter());
        // N.B. increment the counter before creating the checkpoint;
        // The checkpoint creation records a copy of the checkpoint count, and that count should
        // include itself.
        counter.increment();
        let sf_checkpoint = self.out.checkpoint();
        Checkpoint {
            counter,
            sf_checkpoint,
        }
    }

    pub fn restore_checkpoint(&self, checkpoint: &Checkpoint) {
        self.out.restore_checkpoint(&checkpoint.sf_checkpoint)
    }
}

pub struct Lookahead(SourceFormatterLookahead);

impl AstFormatter {
    pub fn capture_lookahead(&self, from: &Checkpoint) -> Lookahead {
        Lookahead(self.out.capture_lookahead(&from.sf_checkpoint))
    }

    pub fn restore_lookahead(&self, lookahead: &Lookahead) {
        self.out.restore_lookahead(&lookahead.0);
    }
}
