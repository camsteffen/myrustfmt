use crate::constraint_writer::TooWideError;

const INDENT_WIDTH: usize = 4;

#[derive(Clone)]
pub struct Constraints {
    pub single_line: bool,
    pub max_width: Option<usize>,
    pub max_width_first_line: Option<usize>,
    pub indent: usize,
    pub newline_budget: Option<usize>,
}

impl Constraints {
    pub fn new(max_width: usize) -> Constraints {
        Constraints {
            newline_budget: None,
            single_line: false,
            max_width: Some(max_width),
            max_width_first_line: None,
            indent: 0,
        }
    }

    pub fn increment_indent(&mut self) {
        self.indent += INDENT_WIDTH;
    }

    pub fn decrement_indent(&mut self) {
        self.indent -= INDENT_WIDTH;
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
}
