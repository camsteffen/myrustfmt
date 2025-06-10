use crate::constraint_writer::checkpoint::ConstraintWriterCheckpoint;
use crate::error_emitter::Checkpoint as BufferedErrorEmitterCheckpoint;
use crate::source_formatter::{Lookahead, SourceFormatter};
use rustc_span::BytePos;

pub struct Checkpoint<'a> {
    error_emitter_checkpoint: Option<BufferedErrorEmitterCheckpoint>,
    owner: &'a SourceFormatter,
    source_pos: BytePos,
    writer_checkpoint: ConstraintWriterCheckpoint,
}

impl Drop for Checkpoint<'_> {
    fn drop(&mut self) {
        if let Some(error_emitter_checkpoint) = self.error_emitter_checkpoint.take() {
            self.owner.error_emitter.commit_checkpoint(
                error_emitter_checkpoint,
            );
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
        Checkpoint {
            error_emitter_checkpoint,
            owner: self,
            source_pos: self.source_reader.pos.get(),
            writer_checkpoint: self.out.checkpoint(),
        }
    }

    pub fn restore_checkpoint(&self, checkpoint: &Checkpoint) {
        let Checkpoint {
            ref error_emitter_checkpoint,
            owner: _,
            source_pos,
            ref writer_checkpoint,
        } = *checkpoint;
        if let Some(error_emitter_checkpoint) = error_emitter_checkpoint {
            self.error_emitter.restore_checkpoint(
                error_emitter_checkpoint,
            );
        }
        self.out.restore_checkpoint(writer_checkpoint);
        self.source_reader.pos.set(source_pos);
    }

    pub fn capture_lookahead(&self, from: &Checkpoint) -> Lookahead {
        let error_buffer = match &from.error_emitter_checkpoint {
            Some(error_emitter_checkpoint) => self.error_emitter.take_from_checkpoint(
                error_emitter_checkpoint,
            ),
            None => Vec::new(),
        };
        let writer_lookahead = self.out.capture_lookahead(&from.writer_checkpoint);
        let source_pos = self.source_reader.pos.get();
        self.source_reader.pos.set(from.source_pos);
        Lookahead {
            error_buffer,
            source_pos,
            writer_lookahead,
        }
    }

    pub fn restore_lookahead(&self, lookahead: Lookahead) {
        let Lookahead {
            error_buffer,
            source_pos,
            writer_lookahead,
        } = lookahead;
        self.error_emitter.push_vec(error_buffer);
        self.out.restore_lookahead(writer_lookahead);
        self.source_reader.pos.set(source_pos);
    }
}
