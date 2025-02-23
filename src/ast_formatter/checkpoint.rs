use crate::ast_formatter::AstFormatter;
use crate::constraints::CheckpointCounter;
use crate::source_formatter::{SourceFormatterCheckpoint, SourceFormatterLookahead};

/// A checkpoint that can be cheaply restored, like an "undo".
/// All checkpoints must be closed *before* the final formatting strategy.
#[must_use]
pub struct Checkpoint {
    #[allow(
        dead_code,
        reason = "uses an Rc to increment the checkpoint count for the lifetime of the checkpoint",
    )]
    counter: CheckpointCounter,
    sf_checkpoint: SourceFormatterCheckpoint,
}

impl AstFormatter {
    pub fn open_checkpoint(&self) -> Checkpoint {
        // N.B. increment the counter before creating the checkpoint;
        // The checkpoint creation records a copy of the checkpoint count, and that count should
        // include itself.
        let counter = self.checkpoint_counter().clone();
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
