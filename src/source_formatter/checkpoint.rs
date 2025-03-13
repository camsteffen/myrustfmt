use rustc_span::BytePos;
use crate::constraint_writer::ConstraintWriterCheckpoint;
use crate::error_emitter::Checkpoint as BufferedErrorEmitterCheckpoint;
use crate::source_formatter::{SourceFormatter, SourceFormatterLookahead};
use crate::util::cell_ext::CellNumberExt;

pub struct Checkpoint<'a> {
    error_emitter_checkpoint: Option<BufferedErrorEmitterCheckpoint>,
    index: u32,
    owner: &'a SourceFormatter,
    source_pos: BytePos,
    writer_checkpoint: ConstraintWriterCheckpoint,
}

impl Drop for Checkpoint<'_> {
    fn drop(&mut self) {
        self.owner.assert_last_checkpoint(self);
        self.owner.checkpoint_count.decrement();
        if let Some(error_emitter_checkpoint) = self.error_emitter_checkpoint.take() {
            self.owner
                .error_emitter
                .commit_checkpoint(error_emitter_checkpoint);
        }
    }
}

impl SourceFormatter {
    pub fn checkpoint(&self) -> Checkpoint<'_> {
        self.checkpoint_inner(true)
    }

    pub fn checkpoint_without_buffer_errors(&self) -> Checkpoint<'_> {
        self.checkpoint_inner(false)
    }

    pub fn checkpoint_inner(&self, buffer_errors: bool) -> Checkpoint<'_> {
        let error_emitter_checkpoint = buffer_errors.then(|| self.error_emitter.checkpoint());
        let index = self.checkpoint_count.get();
        self.checkpoint_count.set(index + 1);
        Checkpoint {
            error_emitter_checkpoint,
            index,
            owner: self,
            source_pos: self.source.pos.get(),
            writer_checkpoint: self.out.checkpoint(),
        }
    }

    fn assert_last_checkpoint(&self, checkpoint: &Checkpoint) {
        assert_eq!(checkpoint.index, self.checkpoint_count.get() - 1);
    }

    pub fn restore_checkpoint(&self, checkpoint: &Checkpoint) {
        self.assert_last_checkpoint(checkpoint);
        let Checkpoint {
            ref error_emitter_checkpoint,
            index: _,
            owner: _,
            source_pos,
            ref writer_checkpoint,
        } = *checkpoint;
        if let Some(error_emitter_checkpoint) = error_emitter_checkpoint {
            self.error_emitter
                .restore_checkpoint(error_emitter_checkpoint);
        }
        self.out.restore_checkpoint(writer_checkpoint);
        self.source.pos.set(source_pos);
    }

    pub fn capture_lookahead(&self, from: &Checkpoint) -> SourceFormatterLookahead {
        self.assert_last_checkpoint(from);
        let error_buffer = match &from.error_emitter_checkpoint {
            Some(error_emitter_checkpoint) => {
                self.error_emitter
                    .take_from_checkpoint(error_emitter_checkpoint)
            }
            None => Vec::new(),
        };
        let writer_lookahead = self.out.capture_lookahead(&from.writer_checkpoint);
        let source_pos = self.source.pos.get();
        self.source.pos.set(from.source_pos);
        SourceFormatterLookahead {
            error_buffer,
            source_pos,
            writer_lookahead,
        }
    }

    pub fn restore_lookahead(&self, lookahead: SourceFormatterLookahead) {
        let SourceFormatterLookahead {
            error_buffer,
            source_pos,
            writer_lookahead,
        } = lookahead;
        self.error_emitter.push_vec(error_buffer);
        self.out.restore_lookahead(writer_lookahead);
        self.source.pos.set(source_pos);
    }
}
