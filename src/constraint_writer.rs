use tracing::{info, instrument};
use crate::constraints::Constraints;

pub struct ConstraintWriter {
    constraints: Constraints,
    buffer: String,
    last_line_start: usize,
}

pub struct WriterSnapshot {
    constraints: Constraints,
    len: usize,
    last_line_start: usize,
}

impl ConstraintWriter {
    pub fn new(constraints: Constraints) -> ConstraintWriter {
        ConstraintWriter {
            constraints,
            buffer: String::new(),
            last_line_start: 0,
        }
    }

    pub fn finish(self) -> String {
        self.buffer
    }

    pub fn constraints(&mut self) -> &mut Constraints {
        &mut self.constraints
    }

    pub fn snapshot(&self) -> WriterSnapshot {
        WriterSnapshot {
            constraints: self.constraints.clone(),
            len: self.buffer.len(),
            last_line_start: self.last_line_start,
        }
    }

    pub fn restore(&mut self, snapshot: &WriterSnapshot) {
        self.constraints = snapshot.constraints.clone();
        self.last_line_start = snapshot.last_line_start;
        self.buffer.truncate(snapshot.len);
    }

    // #[instrument(skip(self))]
    pub fn token(&mut self, token: &str) -> Result<(), TooWideError> {
        self.buffer.push_str(token);
        self.check_width()
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
        Ok(())
    }

    pub fn indent(&mut self) -> Result<(), TooWideError> {
        self.buffer
            .extend(std::iter::repeat_n(' ', self.constraints.indent));
        self.check_width()
    }

    pub fn check_width(&mut self) -> Result<(), TooWideError> {
        match self.remaining_width() {
            Ok(_width) => Ok(()),
            Err(TooWideError) => {
                info!("too wide: \"{}\"", self.last_line());
                Err(TooWideError)
            },
        }
    }

    pub fn remaining_width(&self) -> Result<Option<usize>, TooWideError> {
        self.constraints
            .max_width
            .map(|max_width| {
                max_width
                    .checked_sub(self.last_line_width())
                    .ok_or(TooWideError)
            })
            .transpose()
    }
    
    fn last_line(&self) -> &str {
        &self.buffer[self.last_line_start..]
    }

    // #[instrument(skip(self), ret)]
    pub fn last_line_width(&self) -> usize {
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
