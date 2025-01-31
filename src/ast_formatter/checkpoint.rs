use crate::ast_formatter::AstFormatter;
use crate::source_formatter::SourceFormatterSnapshot;

#[must_use]
pub struct Checkpoint(SourceFormatterSnapshot);

impl AstFormatter {
    pub fn open_checkpoint(&self) -> Checkpoint {
        self.constraints()
            .open_checkpoint_count
            .set(self.constraints().open_checkpoint_count.get() + 1);
        Checkpoint(self.out.snapshot())
    }

    pub fn restore_checkpoint(&self, checkpoint: &Checkpoint) {
        self.out.restore(&checkpoint.0)
    }

    /// All checkpoints must be closed *before* the final formatting strategy
    pub fn close_checkpoint(&self, _: Checkpoint) {
        self.constraints()
            .open_checkpoint_count
            .set(self.constraints().open_checkpoint_count.get() - 1);
    }
}
