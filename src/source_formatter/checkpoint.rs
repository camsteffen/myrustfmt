use crate::constraint_writer::checkpoint::ConstraintWriterCheckpoint;
use crate::error_emitter::Checkpoint as BufferedErrorEmitterCheckpoint;
use crate::source_formatter::SourceFormatter;
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
            self.error_emitter.restore_checkpoint(error_emitter_checkpoint);
        }
        self.out.restore_checkpoint(writer_checkpoint);
        self.source_reader.pos.set(source_pos);
    }
}
