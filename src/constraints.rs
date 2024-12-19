use crate::constraint_writer::TooWideError;
use std::cell::Cell;

const INDENT_WIDTH: usize = 4;

#[derive(Clone)]
pub struct Constraints {
    pub single_line: Cell<bool>,
    pub max_width: Cell<Option<usize>>,
    pub max_width_first_line: Cell<Option<usize>>,
    pub indent: Cell<usize>,
    pub newline_budget: Cell<Option<usize>>,
}

impl Constraints {
    pub fn new(max_width: usize) -> Constraints {
        Constraints {
            indent: Cell::new(0),
            max_width: Cell::new(Some(max_width)),
            max_width_first_line: Cell::new(None),
            newline_budget: Cell::new(None),
            single_line: Cell::new(false),
        }
    }

    pub fn set(&self, other: &Constraints) {
        let Constraints {
            indent,
            max_width,
            max_width_first_line,
            newline_budget,
            single_line,
        } = other;
        self.indent.set(indent.get());
        self.max_width.set(max_width.get());
        self.max_width_first_line.set(max_width_first_line.get());
        self.newline_budget.set(newline_budget.get());
        self.single_line.set(single_line.get());
    }

    pub fn increment_indent(&self) {
        self.indent.set(self.indent.get() + INDENT_WIDTH);
    }

    pub fn decrement_indent(&self) {
        self.indent.set(self.indent.get() - INDENT_WIDTH);
    }

    pub fn add_max_width(&self, len: usize) {
        if let Some(max_width) = self.max_width.get() {
            self.max_width.set(Some(max_width + len));
        }
    }

    pub fn sub_max_width(&self, len: usize) -> Result<(), TooWideError> {
        if let Some(max_width) = self.max_width.get() {
            self.max_width
                .set(Some(max_width.checked_sub(len).ok_or(TooWideError)?));
        }
        Ok(())
    }
}
