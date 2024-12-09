use tracing::instrument;

const INDENT_WIDTH: usize = 4;

pub struct ConstraintWriter {
    buffer: String,
    last_line_start: usize,
    max_width: Option<usize>,
    indent: usize,
    constraints: Vec<Constraint>,
}

pub enum Constraint {
    SingleLine,
    SingleLineLimitWidth { pos: usize },
}

pub struct WriterSnapshot {
    len: usize,
    indent: usize,
    last_line_start: usize,
}

impl ConstraintWriter {
    pub fn new(max_width: usize) -> ConstraintWriter {
        ConstraintWriter {
            buffer: String::new(),
            last_line_start: 0,
            max_width: Some(max_width),
            indent: 0,
            constraints: Vec::new(),
        }
    }

    pub fn finish(self) -> String {
        self.buffer
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn current_indent(&self) -> usize {
        self.indent
    }

    pub fn set_indent(&mut self, indent: usize) {
        self.indent = indent;
    }

    pub fn snapshot(&self) -> WriterSnapshot {
        WriterSnapshot {
            len: self.buffer.len(),
            indent: self.indent,
            last_line_start: self.last_line_start,
        }
    }

    pub fn restore(&mut self, snapshot: &WriterSnapshot) {
        self.indent = snapshot.indent;
        self.last_line_start = snapshot.last_line_start;
        self.buffer.truncate(snapshot.len);
    }

    pub fn push_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    pub fn pop_constraint(&mut self) {
        self.constraints.pop();
    }

    pub fn add_max_width(&mut self, len: usize) {
        if let Some(max_width) = &mut self.max_width {
            *max_width += len;
        }
    }

    pub fn sub_max_width(&mut self, len: usize) -> Result<(), TooWideError> {
        if let Some(max_width) = &mut self.max_width {
            *max_width = max_width.checked_sub(len).ok_or(TooWideError)?;
        }
        Ok(())
    }

    pub fn token(&mut self, token: &str) -> Result<(), TooWideError> {
        for constraint in &self.constraints {
            match constraint {
                Constraint::SingleLine => {}
                Constraint::SingleLineLimitWidth { pos } => {
                    if token.len() > pos - self.buffer.len() {
                        return Err(TooWideError);
                    }
                }
            }
        }
        self.reserve(token.len())?;
        self.buffer.push_str(token);
        Ok(())
    }

    pub fn newline(&mut self) -> Result<(), NewlineNotAllowedError> {
        for constraint in &self.constraints {
            match constraint {
                Constraint::SingleLine | Constraint::SingleLineLimitWidth { .. } => {
                    return Err(NewlineNotAllowedError);
                }
            }
        }
        self.buffer.push('\n');
        self.last_line_start = self.buffer.len();
        Ok(())
    }

    pub fn increment_indent(&mut self) {
        self.indent += INDENT_WIDTH;
    }

    pub fn decrement_indent(&mut self) {
        self.indent -= INDENT_WIDTH;
    }

    pub fn indent(&mut self) -> Result<(), TooWideError> {
        self.reserve(self.indent)?;
        self.buffer.extend(std::iter::repeat_n(' ', self.indent));
        Ok(())
    }

    #[instrument(skip(self), ret, fields(out = self.buffer))]
    fn reserve(&mut self, len: usize) -> Result<(), TooWideError> {
        if let Some(max_width) = self.max_width {
            if len > max_width - self.last_line_width() {
                return Err(TooWideError);
            }
        }
        Ok(())
    }

    #[instrument(skip(self), ret)]
    fn last_line_width(&self) -> usize {
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
