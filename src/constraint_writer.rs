use crate::constraints::{CheckpointCounter, Constraints, MultiLineShape, OwnedConstraints};
use crate::error::{
    ConstraintError, ConstraintErrorKind, NewlineNotAllowedError, WidthLimitExceededError,
};
use crate::error_emitter::ErrorEmitter;
use crate::util::cell_ext::CellExt;
use std::cell::Cell;
use std::rc::Rc;

pub struct ConstraintWriter {
    checkpoint_counter: Rc<CheckpointCounter>,
    constraints: OwnedConstraints,
    buffer: Cell<String>,
    error_emitter: Rc<ErrorEmitter>,
    last_line_start: Cell<usize>,
    last_width_exceeded_line: Cell<Option<u32>>,
    line: Cell<u32>,
}

pub struct ConstraintWriterCheckpoint {
    len: usize,
    self_checkpoint: ConstraintWriterSelfCheckpoint,
}

// todo rename
pub struct ConstraintWriterSelfCheckpoint {
    line: u32,
    last_line_start: usize,
    last_width_exceeded_line: Option<u32>,
    #[cfg(debug_assertions)]
    checkpoint_count: u32,
    #[cfg(debug_assertions)]
    constraints: Rc<Constraints>,
}

pub struct ConstraintWriterLookahead {
    buf_segment: String,
    checkpoint: ConstraintWriterSelfCheckpoint,
}

pub struct ConstraintWriterResult {
    pub formatted: String,
    pub exceeded_max_width: bool,
}

impl ConstraintWriter {
    pub fn new(
        constraints: OwnedConstraints,
        error_emitter: Rc<ErrorEmitter>,
        capacity: usize,
    ) -> ConstraintWriter {
        ConstraintWriter {
            checkpoint_counter: Rc::new(CheckpointCounter::default()),
            constraints,
            buffer: Cell::new(String::with_capacity(capacity)),
            error_emitter,
            last_line_start: Cell::new(0),
            last_width_exceeded_line: Cell::new(None),
            line: Cell::new(0),
        }
    }

    pub fn finish(self) -> String {
        self.buffer.into_inner()
    }

    pub fn checkpoint_counter(&self) -> &Rc<CheckpointCounter> {
        &self.checkpoint_counter
    }

    pub fn constraints(&self) -> &OwnedConstraints {
        &self.constraints
    }

    pub fn len(&self) -> usize {
        self.buffer.with_taken(|b| b.len())
    }

    pub fn line(&self) -> u32 {
        self.line.get()
    }

    pub fn checkpoint(&self) -> ConstraintWriterCheckpoint {
        ConstraintWriterCheckpoint {
            self_checkpoint: self.self_checkpoint(),
            len: self.len(),
        }
    }

    pub fn self_checkpoint(&self) -> ConstraintWriterSelfCheckpoint {
        let Self {
            ref checkpoint_counter,
            ref constraints,
            buffer: _,
            error_emitter: _,
            ref last_line_start,
            ref last_width_exceeded_line,
            ref line,
        } = *self;
        ConstraintWriterSelfCheckpoint {
            line: line.get(),
            last_line_start: last_line_start.get(),
            last_width_exceeded_line: last_width_exceeded_line.get(),
            #[cfg(debug_assertions)]
            checkpoint_count: checkpoint_counter.count(),
            #[cfg(debug_assertions)]
            constraints: Rc::clone(&constraints.borrow()),
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
            last_line_start,
            last_width_exceeded_line,
            line,
            #[cfg(debug_assertions)]
            checkpoint_count,
            #[cfg(debug_assertions)]
            ref constraints,
        } = *checkpoint;
        #[cfg(debug_assertions)]
        {
            assert_eq!(&*self.constraints.borrow(), constraints);
            assert_eq!(self.checkpoint_counter.count(), checkpoint_count);
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

    pub fn restore_lookahead(&self, lookahead: &ConstraintWriterLookahead) {
        self.buffer.with_taken(|b| b.push_str(&lookahead.buf_segment));
        self.restore_self_checkpoint(&lookahead.checkpoint);
    }

    pub fn token(&self, token: &str) -> Result<(), ConstraintError> {
        self.buffer.with_taken(|b| b.push_str(token));
        self.check_width_constraints()
    }

    pub fn write_possibly_multiline(&self, source: &str) -> Result<(), ConstraintError> {
        for c in source.chars() {
            if c == '\n' {
                self.newline()?;
            } else {
                self.buffer.with_taken(|b| b.push(c));
                self.check_width_constraints()?;
            }
        }
        Ok(())
    }

    pub fn newline(&self) -> Result<(), NewlineNotAllowedError> {
        if matches!(self.constraints.borrow().multi_line, MultiLineShape::SingleLine) {
            return Err(NewlineNotAllowedError);
        }
        self.buffer.with_taken(|b| b.push('\n'));
        self.last_line_start.set(self.len());
        self.line.set(self.line.get() + 1);
        Ok(())
    }

    pub fn spaces(&self, count: u32) {
        self.buffer.with_taken(|b| b.extend((0..count).map(|_| ' ')));
    }

    pub fn check_width_constraints(&self) -> Result<(), ConstraintError> {
        let _: WidthLimitExceededError = match self.remaining_width() {
            None | Some(Ok(_)) => return Ok(()),
            Some(Err(e)) => e,
        };
        // If there is a fallback formatting strategy, then raise an error to trigger the
        // fallback. Otherwise, emit an error and keep going.
        if self.checkpoint_counter.count() > 0 {
            Err(ConstraintError::new(
                ConstraintErrorKind::WidthLimitExceeded,
                #[cfg(debug_assertions)]
                self.checkpoint_counter.take_backtrace(),
            ))
        } else {
            let line = self.line.get();
            if self.last_width_exceeded_line.get() != Some(line) {
                self.last_width_exceeded_line.set(Some(line));
                self.with_last_line(|line_str| {
                    self.error_emitter.emit_width_exceeded(line, line_str)
                });
            }
            Ok(())
        }
    }

    pub fn current_max_width(&self) -> Option<u32> {
        self.constraints.borrow().max_width_at(self.line())
    }

    pub fn remaining_width(&self) -> Option<Result<u32, WidthLimitExceededError>> {
        self.current_max_width().map(|max_width| {
            max_width
                .checked_sub(self.last_line_len().try_into().unwrap())
                .ok_or(WidthLimitExceededError)
        })
    }

    pub fn with_last_line<T>(&self, f: impl FnOnce(&str) -> T) -> T {
        self.buffer.with_taken(|b| f(&b[self.last_line_start.get()..]))
    }

    pub fn last_line_len(&self) -> u32 {
        (self.len() - self.last_line_start.get())
            .try_into()
            .unwrap()
    }

    // for debugging
    // #[allow(unused)]
    pub fn with_taken_buffer(&self, f: impl FnOnce(&mut String)) {
        self.buffer.with_taken(f)
    }
}
