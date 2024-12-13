use crate::constraints::Constraints;
use tracing::{info, instrument};

pub struct ConstraintWriter {
    constraints: Constraints,
    buffer: String,
    last_line_start: usize,
    line: usize,
}

pub struct ConstraintWriterSnapshot {
    constraints: Constraints,
    line: usize,
    len: usize,
    last_line_start: usize,
}

impl ConstraintWriter {
    pub fn new(constraints: Constraints) -> ConstraintWriter {
        ConstraintWriter {
            constraints,
            buffer: String::new(),
            last_line_start: 0,
            line: 0,
        }
    }

    pub fn finish(self) -> String {
        self.buffer
    }

    pub fn constraints(&mut self) -> &mut Constraints {
        &mut self.constraints
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn snapshot(&self) -> ConstraintWriterSnapshot {
        let Self {
            ref constraints,
            ref buffer,
            last_line_start,
            line,
        } = *self;
        ConstraintWriterSnapshot {
            constraints: constraints.clone(),
            line,
            len: buffer.len(),
            last_line_start,
        }
    }

    pub fn restore(&mut self, snapshot: &ConstraintWriterSnapshot) {
        let ConstraintWriterSnapshot {
            ref constraints,
            last_line_start,
            len,
            line,
        } = *snapshot;
        self.constraints = constraints.clone();
        self.last_line_start = last_line_start;
        self.line = line;
        self.buffer.truncate(len);
    }

    // #[instrument(skip(self))]
    pub fn token(&mut self, token: &str) -> Result<(), TooWideError> {
        self.buffer.push_str(token);
        self.check_width_constraints()
    }

    pub fn write_unchecked(&mut self, source: &str) {
        self.buffer.push_str(source);
    }

    pub fn newline(&mut self) -> Result<(), NewlineNotAllowedError> {
        if self.constraints.single_line {
            return Err(NewlineNotAllowedError);
        }
        self.buffer.push('\n');
        self.last_line_start = self.buffer.len();
        self.line += 1;
        self.constraints.max_width_first_line = None;
        Ok(())
    }

    pub fn indent(&mut self) -> Result<(), TooWideError> {
        self.buffer
            .extend(std::iter::repeat_n(' ', self.constraints.indent));
        self.check_width_constraints()
    }

    pub fn check_width_constraints(&mut self) -> Result<(), TooWideError> {
        match self.remaining_width() {
            Ok(_width) => Ok(()),
            Err(TooWideError) => {
                info!("too wide: \"{}\"", self.last_line());
                Err(TooWideError)
            }
        }
    }

    pub fn max_width(&self) -> Option<usize> {
        let a = self.constraints.max_width?;
        let b = self.constraints.max_width_first_line?;
        Some(a.min(b))
    }

    pub fn remaining_width(&self) -> Result<Option<usize>, TooWideError> {
        self.max_width()
            .map(|max_width| {
                max_width
                    .checked_sub(self.last_line_len())
                    .ok_or(TooWideError)
            })
            .transpose()
    }

    fn last_line(&self) -> &str {
        &self.buffer[self.last_line_start..]
    }

    // #[instrument(skip(self), ret)]
    pub fn last_line_len(&self) -> usize {
        self.buffer.len() - self.last_line_start
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ConstraintError {
    NewlineNotAllowed,
    TooWide,
}

#[derive(Debug)]
pub struct NewlineNotAllowedError;

#[derive(Debug)]
pub struct TooWideError;

impl From<NewlineNotAllowedError> for ConstraintError {
    fn from(_: NewlineNotAllowedError) -> Self {
        ConstraintError::NewlineNotAllowed
    }
}

impl From<TooWideError> for ConstraintError {
    fn from(_: TooWideError) -> Self {
        ConstraintError::TooWide
    }
}
