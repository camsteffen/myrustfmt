pub mod checkpoint;

use crate::constraints::{Constraints, Shape};
use crate::error::{
    ConstraintError, ConstraintErrorKind, FormatResult, NewlineNotAllowedError,
    WidthLimitExceededError,
};
use crate::error_emitter::BufferedErrorEmitter;
use crate::num::HPos;
use crate::util::cell_ext::{CellExt, CellNumberExt};
use std::cell::Cell;
use std::panic::Location;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum ConstraintRecoveryMode {
    Nothing,
    Newline,
    MaxWidth { line: u32 },
    // MultiLineComment,
}

pub struct ConstraintWriter {
    constraints: Constraints,
    buffer: Cell<String>,
    constraint_recovery_mode: Cell<ConstraintRecoveryMode>,
    errors: Rc<BufferedErrorEmitter>,
    last_line_start: Cell<usize>,
    last_width_exceeded_line: Cell<Option<u32>>,
    line: Cell<u32>,
}

impl ConstraintWriter {
    pub fn new(
        constraints: Constraints,
        errors: Rc<BufferedErrorEmitter>,
        capacity: usize,
    ) -> ConstraintWriter {
        ConstraintWriter {
            constraints,
            buffer: Cell::new(String::with_capacity(capacity)),
            constraint_recovery_mode: Cell::new(ConstraintRecoveryMode::Nothing),
            errors,
            last_line_start: Cell::new(0),
            last_width_exceeded_line: Cell::new(None),
            line: Cell::new(0),
        }
    }

    pub fn finish(self) -> String {
        self.buffer.into_inner()
    }

    pub fn constraints(&self) -> &Constraints {
        &self.constraints
    }

    pub fn len(&self) -> usize {
        self.buffer.with_taken(|b| b.len())
    }

    pub fn line(&self) -> u32 {
        self.line.get()
    }

    pub fn with_constraint_recovery_mode_max<T>(
        &self,
        mode: ConstraintRecoveryMode,
        scope: impl FnOnce() -> T,
    ) -> T {
        if self.constraint_recovery_mode.get() >= mode {
            return scope();
        }
        self.constraint_recovery_mode.with_replaced(mode, scope)
    }

    pub fn with_enforce_max_width<T>(&self, scope: impl FnOnce() -> T) -> T {
        self.with_constraint_recovery_mode_max(self.max_recovery_mode(), scope)
    }

    pub fn has_any_constraint_recovery(&self) -> bool {
        match self.constraint_recovery_mode.get() {
            ConstraintRecoveryMode::Nothing => false,
            // todo doesn't make sense
            ConstraintRecoveryMode::Newline => true,
            ConstraintRecoveryMode::MaxWidth { .. } => true,
        }
    }

    pub fn is_enforcing_width(&self) -> bool {
        if self
            .constraints
            .width_limit()
            .is_some_and(|limit| limit.is_applicable(self.line()))
        {
            return true;
        }
        match self.constraint_recovery_mode.get() {
            ConstraintRecoveryMode::Nothing => false,
            ConstraintRecoveryMode::Newline => false,
            ConstraintRecoveryMode::MaxWidth { line } => line == self.line(),
        }
    }

    pub fn token(&self, token: &str) -> FormatResult {
        self.buffer.with_taken(|b| b.push_str(token));
        self.check_width_constraints()
    }

    pub fn write_possibly_multiline(&self, source: &str) -> FormatResult {
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
        if matches!(self.constraints.shape(), Shape::SingleLine) {
            return Err(NewlineNotAllowedError);
        }
        self.buffer.with_taken(|b| b.push('\n'));
        self.last_line_start.set(self.len());
        self.line.increment();
        Ok(())
    }

    pub fn spaces(&self, count: HPos) {
        self.buffer.with_taken(|b| b.extend((0..count).map(|_| ' ')));
    }

    pub fn check_width_constraints(&self) -> FormatResult {
        if self.remaining_width().is_ok() {
            return Ok(());
        }
        // If there is a fallback formatting strategy, then raise an error to trigger the
        // fallback. Otherwise, emit an error and keep going.
        if self.is_enforcing_width() {
            Err(
                ConstraintError::new(ConstraintErrorKind::WidthLimitExceeded),
            )
        } else {
            let line = self.line.get();
            if self.last_width_exceeded_line.get() != Some(line) {
                self.last_width_exceeded_line.set(Some(line));
                self.errors.max_width_exceeded(line);
            }
            Ok(())
        }
    }

    pub fn current_max_width(&self) -> HPos {
        self.constraints.max_width_at(self.line())
    }

    pub fn remaining_width(&self) -> Result<HPos, WidthLimitExceededError> {
        self.current_max_width()
            .checked_sub(self.last_line_len().try_into().unwrap())
            .ok_or(WidthLimitExceededError)
    }

    pub fn with_last_line<T>(&self, f: impl FnOnce(&str) -> T) -> T {
        self.buffer.with_taken(|b| f(&b[self.last_line_start.get()..]))
    }

    pub fn last_line_len(&self) -> HPos {
        (self.len() - self.last_line_start.get())
            .try_into()
            .expect("line length exceeds HPos::MAX")
    }

    pub fn with_taken_buffer(&self, f: impl FnOnce(&mut String)) {
        self.buffer.with_taken(f)
    }

    // todo rename
    pub fn max_recovery_mode(&self) -> ConstraintRecoveryMode {
        ConstraintRecoveryMode::MaxWidth { line: self.line() }
    }

    #[track_caller]
    #[allow(unused)]
    pub fn debug_buffer(&self) {
        let location = Location::caller();
        self.with_taken_buffer(|b| {
            eprintln!("[{location}] buffer:\n{b}\n\n");
        });
    }
}
