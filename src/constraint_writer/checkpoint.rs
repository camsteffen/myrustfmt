use crate::constraint_writer::ConstraintWriter;
use crate::constraints::ConstraintsCheckpoint;
use crate::num::VSize;
use crate::util::cell_ext::CellExt;

pub struct ConstraintWriterCheckpoint {
    len: usize,
    self_checkpoint: ConstraintWriterSelfCheckpoint,
}

// todo rename
#[derive(Debug)]
pub struct ConstraintWriterSelfCheckpoint {
    constraints_checkpoint: ConstraintsCheckpoint,
    line: VSize,
    last_line_start: usize,
    last_width_exceeded_line: Option<VSize>,
}

impl ConstraintWriter {
    pub fn checkpoint(&self) -> ConstraintWriterCheckpoint {
        ConstraintWriterCheckpoint {
            self_checkpoint: self.self_checkpoint(),
            len: self.len(),
        }
    }

    pub fn self_checkpoint(&self) -> ConstraintWriterSelfCheckpoint {
        let Self {
            ref constraints,
            ref last_line_start,
            ref last_width_exceeded_line,
            ref line,
            buffer: _,
            errors: _,
        } = *self;
        ConstraintWriterSelfCheckpoint {
            constraints_checkpoint: constraints.checkpoint(),
            line: line.get(),
            last_line_start: last_line_start.get(),
            last_width_exceeded_line: last_width_exceeded_line.get(),
        }
    }

    pub fn restore_checkpoint(&self, checkpoint: &ConstraintWriterCheckpoint) {
        let ConstraintWriterCheckpoint {
            ref self_checkpoint,
            len,
        } = *checkpoint;
        self.restore_self_checkpoint(self_checkpoint);
        self.buffer.with_taken(|b| b.truncate(len));
    }

    pub fn restore_self_checkpoint(&self, checkpoint: &ConstraintWriterSelfCheckpoint) {
        let ConstraintWriterSelfCheckpoint {
            ref constraints_checkpoint,
            last_line_start,
            last_width_exceeded_line,
            line,
        } = *checkpoint;
        self.constraints.restore_checkpoint(constraints_checkpoint);
        self.last_line_start.set(last_line_start);
        self.last_width_exceeded_line.set(last_width_exceeded_line);
        self.line.set(line);
    }
}
