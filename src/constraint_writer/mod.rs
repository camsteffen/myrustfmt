pub mod checkpoint;

use crate::constraints::{Constraints, Shape};
use crate::error::{
    ConstraintError, ConstraintErrorKind, FormatResult, NewlineNotAllowedError,
    WidthLimitExceededError,
};
use crate::error_emitter::BufferedErrorEmitter;
use crate::num::HSize;
use crate::util::cell_ext::{CellExt, CellNumberExt};
use std::cell::Cell;
use std::panic::Location;
use std::rc::Rc;

/// Specifies which constraints, when they are violated, may be recovered by falling back to another
/// formatting strategy. When a recoverable constraint is violated, a `ConstraintError` is returned
/// and is expected to trigger the next strategy. When it is not recoverable, an error is emitted to
/// the user.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum RecoverableConstraints {
    /// Don't recover from anything.
    Nothing,
    /// Recover from disallowed newline characters.
    Newline,
    /// Recover from exceeding width constraints, but only on the specified line.
    /// When in scope, this also includes newline constraints.
    Width { line: u32 },
}

impl RecoverableConstraints {
    pub fn is_nothing(self) -> bool {
        matches!(self, RecoverableConstraints::Nothing)
    }
}

pub struct ConstraintWriter {
    constraints: Constraints,
    buffer: Cell<String>,
    errors: Rc<BufferedErrorEmitter>,
    last_line_start: Cell<usize>,
    last_width_exceeded_line: Cell<Option<u32>>,
    line: Cell<u32>,
    recoverable_constraints: Cell<RecoverableConstraints>,
}

impl ConstraintWriter {
    pub fn new(
        max_width: HSize,
        errors: Rc<BufferedErrorEmitter>,
        capacity: usize,
    ) -> ConstraintWriter {
        ConstraintWriter {
            constraints: Constraints::new(max_width),
            buffer: Cell::new(String::with_capacity(capacity)),
            errors,
            last_line_start: Cell::new(0),
            last_width_exceeded_line: Cell::new(None),
            line: Cell::new(0),
            recoverable_constraints: Cell::new(RecoverableConstraints::Nothing),
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

    // todo make sure any math using two values of this are guaranteed to be on the same line
    pub fn col(&self) -> HSize {
        (self.len() - self.last_line_start.get())
            .try_into()
            .expect("line length exceeds HSize::MAX")
    }

    pub fn line_col(&self) -> (u32, HSize) {
        (self.line(), self.col())
    }

    pub fn with_recoverable_constraints<T>(
        &self,
        recoverable_constraints: RecoverableConstraints,
        scope: impl FnOnce() -> T,
    ) -> T {
        if recoverable_constraints <= self.recoverable_constraints.get() {
            return scope();
        }
        self.recoverable_constraints
            .with_replaced(recoverable_constraints, scope)
    }

    pub fn with_recoverable_width<T>(&self, scope: impl FnOnce() -> T) -> T {
        self.with_recoverable_constraints(self.recoverable_width(), scope)
    }

    pub fn recoverable_constraints(&self) -> RecoverableConstraints {
        self.recoverable_constraints.get()
    }

    pub fn is_enforcing_width(&self) -> bool {
        if self
            .constraints
            .width_limit()
            .is_some_and(|limit| limit.is_applicable(self.line()))
        {
            return true;
        }
        match self.recoverable_constraints.get() {
            RecoverableConstraints::Nothing => false,
            RecoverableConstraints::Newline => false,
            RecoverableConstraints::Width { line } => line == self.line(),
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

    pub fn spaces(&self, count: HSize) {
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
            if self.last_width_exceeded_line.replace(Some(line)) != Some(line) {
                self.errors.max_width_exceeded(line);
            }
            Ok(())
        }
    }

    pub fn current_max_width(&self) -> HSize {
        self.constraints.max_width_at(self.line())
    }

    pub fn remaining_width(&self) -> Result<HSize, WidthLimitExceededError> {
        self.current_max_width()
            .checked_sub(self.col().try_into().unwrap())
            .ok_or(WidthLimitExceededError)
    }

    pub fn with_last_line<T>(&self, f: impl FnOnce(&str) -> T) -> T {
        self.buffer.with_taken(|b| f(&b[self.last_line_start.get()..]))
    }

    pub fn with_taken_buffer(&self, f: impl FnOnce(&mut String)) {
        self.buffer.with_taken(f)
    }

    pub fn recoverable_width(&self) -> RecoverableConstraints {
        RecoverableConstraints::Width { line: self.line() }
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
