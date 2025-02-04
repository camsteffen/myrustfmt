use crate::ast_formatter::AstFormatter;
use crate::source_formatter::{SourceFormatterCheckpoint, SourceFormatterLookahead};

/// A checkpoint that can be cheaply restored, like an "undo".
/// All checkpoints must be closed *before* the final formatting strategy.
#[must_use]
pub struct Checkpoint(SourceFormatterCheckpoint);

pub struct Lookahead(SourceFormatterLookahead);

impl AstFormatter {
    pub fn open_checkpoint(&self) -> Checkpoint {
        self.constraints()
            .open_checkpoint_count
            .set(self.constraints().open_checkpoint_count.get() + 1);
        Checkpoint(self.out.checkpoint())
    }

    pub fn restore_checkpoint(&self, checkpoint: &Checkpoint) {
        self.out.restore_checkpoint(&checkpoint.0)
    }

    pub fn close_checkpoint(&self, _: Checkpoint) {
        self.constraints()
            .open_checkpoint_count
            .set(self.constraints().open_checkpoint_count.get() - 1);
    }
    
    pub fn capture_lookahead(&self, from: &Checkpoint) -> Lookahead {
        Lookahead(self.out.capture_lookahead(&from.0))
    }
    
    pub fn restore_lookahead(&self, lookahead: &Lookahead) {
        self.out.restore_lookahead(&lookahead.0);
    }
}
