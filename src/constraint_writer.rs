use crate::ast_formatter::FormatModuleResult;
use crate::constraints::Constraints;
use crate::error::{ConstraintError, NewlineNotAllowedError, WidthLimitExceededError};
use crate::error_emitter::ErrorEmitter;
use crate::util::option::merge_options;
use std::cell::Cell;
use std::rc::Rc;

pub struct ConstraintWriter {
    constraints: Constraints,
    buffer: Cell<String>,
    error_emitter: Rc<ErrorEmitter>,
    /// True if max width was ever exceeded (and there was no fallback)
    exceeded_max_width: Cell<bool>,
    last_line_start: Cell<usize>,
    last_width_exceeded_line: Cell<Option<u32>>,
    line: Cell<u32>,
}

pub struct ConstraintWriterSnapshot {
    constraints: Constraints,
    line: u32,
    len: usize,
    last_line_start: usize,
    last_width_exceeded_line: Option<u32>,
}

pub struct ConstraintWriterResult {
    pub formatted: String,
    pub exceeded_max_width: bool,
}

impl ConstraintWriter {
    pub fn new(constraints: Constraints, error_emitter: Rc<ErrorEmitter>) -> ConstraintWriter {
        ConstraintWriter {
            constraints,
            buffer: Cell::new(String::new()),
            error_emitter,
            exceeded_max_width: Cell::new(false),
            last_line_start: Cell::new(0),
            last_width_exceeded_line: Cell::new(None),
            line: Cell::new(0),
        }
    }

    pub fn finish(self) -> FormatModuleResult {
        FormatModuleResult {
            formatted: self.buffer.into_inner(),
            exceeded_max_width: self.exceeded_max_width.get(),
        }
    }

    pub fn constraints(&self) -> &Constraints {
        &self.constraints
    }

    pub fn len(&self) -> usize {
        self.with_buffer(|b| b.len())
    }

    pub fn line(&self) -> u32 {
        self.line.get()
    }

    fn with_buffer<T>(&self, f: impl FnOnce(&mut String) -> T) -> T {
        let mut buffer = self.buffer.take();
        let out = f(&mut buffer);
        self.buffer.set(buffer);
        out
    }

    pub fn snapshot(&self) -> ConstraintWriterSnapshot {
        let Self {
            ref constraints,
            buffer: _,
            error_emitter: _,
            // exceeded_max_width should only be changed when there is no fallback
            exceeded_max_width: _,
            ref last_line_start,
            ref last_width_exceeded_line,
            ref line,
        } = *self;
        ConstraintWriterSnapshot {
            constraints: constraints.clone(),
            line: line.get(),
            len: self.len(),
            last_line_start: last_line_start.get(),
            last_width_exceeded_line: last_width_exceeded_line.get(),
        }
    }

    pub fn restore(&self, snapshot: &ConstraintWriterSnapshot) {
        let ConstraintWriterSnapshot {
            ref constraints,
            last_line_start,
            last_width_exceeded_line,
            len,
            line,
        } = *snapshot;
        // todo do we really need to restore constraints if modifications are always scoped?
        //   otherwise make sure each field is actually scoped
        assert_eq!(&self.constraints, constraints);
        self.constraints.set(constraints);
        self.last_line_start.set(last_line_start);
        self.last_width_exceeded_line.set(last_width_exceeded_line);
        self.line.set(line);
        self.with_buffer(|b| b.truncate(len));
    }

    pub fn token(&self, token: &str) -> Result<(), WidthLimitExceededError> {
        self.with_buffer(|b| b.push_str(token));
        self.check_width_constraints()
    }

    pub fn write_possibly_multiline(&self, source: &str) -> Result<(), ConstraintError> {
        for c in source.chars() {
            if c == '\n' {
                self.newline()?;
            } else {
                self.with_buffer(|b| b.push(c));
                self.check_width_constraints()?;
            }
        }
        Ok(())
    }

    pub fn newline(&self) -> Result<(), NewlineNotAllowedError> {
        if self.constraints.single_line.get() {
            return Err(NewlineNotAllowedError);
        }
        self.with_buffer(|b| b.push('\n'));
        self.last_line_start.set(self.len());
        self.line.set(self.line.get() + 1);
        self.constraints.max_width_for_line.set(None);
        Ok(())
    }

    pub fn indent(&self) -> Result<(), WidthLimitExceededError> {
        self.with_buffer(|b| {
            b.extend(std::iter::repeat_n(' ', self.constraints.indent.get() as usize))
        });
        self.check_width_constraints()
    }

    pub fn check_width_constraints(&self) -> Result<(), WidthLimitExceededError> {
        match self.remaining_width() {
            None | Some(Ok(_)) => Ok(()),
            Some(Err(WidthLimitExceededError { .. })) => {
                if self.constraints.has_open_checkpoints() {
                    Err(WidthLimitExceededError)
                } else {
                    let line = self.line.get();
                    if self.last_width_exceeded_line.get() != Some(line) {
                        self.exceeded_max_width.set(true);
                        self.last_width_exceeded_line.set(Some(line));
                        self.with_last_line(|line_str| {
                            self.error_emitter.emit_width_exceeded(line, line_str)
                        });
                    }
                    Ok(())
                }
            }
        }
    }

    pub fn max_width(&self) -> Option<u32> {
        let max_width = self.constraints.max_width.get();
        let max_width_for_current_line = self
            .constraints
            .max_width_for_line
            .get()
            .filter(|m| m.line == self.line())
            .map(|m| m.max_width);
        merge_options(max_width, max_width_for_current_line, u32::min)
    }

    pub fn remaining_width(&self) -> Option<Result<u32, WidthLimitExceededError>> {
        self.max_width().map(|max_width| {
            max_width
                .checked_sub(self.last_line_len() as u32)
                .ok_or(WidthLimitExceededError)
        })
    }

    pub fn with_last_line<T>(&self, f: impl FnOnce(&str) -> T) -> T {
        self.with_buffer(|b| f(&b[self.last_line_start.get()..]))
    }

    pub fn last_line_len(&self) -> usize {
        self.len() - self.last_line_start.get()
    }
    
    pub fn split_off(&self, at: usize) -> String {
        self.with_buffer(|b| b.split_off(at))
    }
}
