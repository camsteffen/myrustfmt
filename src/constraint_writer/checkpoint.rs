use crate::constraint_writer::{ConstraintRecoveryMode, ConstraintWriter};
use crate::constraints::Constraints;
use crate::util::cell_ext::CellExt;

pub struct ConstraintWriterCheckpoint {
    len: usize,
    self_checkpoint: ConstraintWriterSelfCheckpoint,
}

// todo rename
#[derive(Debug)]
pub struct ConstraintWriterSelfCheckpoint {
    #[cfg(debug_assertions)]
    constraint_recovery_mode: ConstraintRecoveryMode,
    line: u32,
    last_line_start: usize,
    last_width_exceeded_line: Option<u32>,
    #[cfg(debug_assertions)]
    constraints: Constraints,
}

#[derive(Debug)]
pub struct ConstraintWriterLookahead {
    buf_segment: String,
    checkpoint: ConstraintWriterSelfCheckpoint,
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
            buffer: _,
            #[cfg(debug_assertions)]
            ref constraint_recovery_mode,
            #[cfg(debug_assertions)]
            ref constraints,
            errors: _,
            ref last_line_start,
            ref last_width_exceeded_line,
            ref line,
            ..
        } = *self;
        ConstraintWriterSelfCheckpoint {
            line: line.get(),
            last_line_start: last_line_start.get(),
            last_width_exceeded_line: last_width_exceeded_line.get(),
            #[cfg(debug_assertions)]
            constraint_recovery_mode: constraint_recovery_mode.get(),
            #[cfg(debug_assertions)]
            constraints: constraints.clone(),
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
            #[cfg(debug_assertions)]
            constraint_recovery_mode,
            last_line_start,
            last_width_exceeded_line,
            line,
            #[cfg(debug_assertions)]
            ref constraints,
        } = *checkpoint;
        #[cfg(debug_assertions)]
        {
            assert_eq!(
                self.constraint_recovery_mode.get(),
                constraint_recovery_mode
            );
            assert_eq!(&self.constraints, constraints);
        }
        self.last_line_start.set(last_line_start);
        self.last_width_exceeded_line.set(last_width_exceeded_line);
        self.line.set(line);
    }

    pub fn capture_lookahead(
        &self,
        from: &ConstraintWriterCheckpoint,
    ) -> ConstraintWriterLookahead {
        let checkpoint = self.self_checkpoint();
        let buf_segment = self.buffer.with_taken(|b| b.split_off(from.len));
        self.restore_self_checkpoint(&from.self_checkpoint);
        ConstraintWriterLookahead {
            buf_segment,
            checkpoint,
        }
    }

    pub fn restore_lookahead(&self, lookahead: ConstraintWriterLookahead) {
        self.buffer.with_taken(|b| b.push_str(&lookahead.buf_segment));
        self.restore_self_checkpoint(&lookahead.checkpoint);
    }
}
