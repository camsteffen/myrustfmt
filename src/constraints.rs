use crate::config::Config;
use crate::error::{FormatResult, WidthLimitExceededError};
use std::cell::Cell;

pub const INDENT_WIDTH: usize = 4;

#[derive(Clone, Copy, Debug)]
pub struct MaxWidthForLine {
    pub line: u32,
    pub max_width: u32,
}

#[derive(Clone, Debug)]
pub struct Constraints {
    pub single_line: Cell<bool>,
    pub max_width: Cell<Option<u32>>,
    /// Used to set the max width for the current line, so it no longer applies after a newline
    /// character is printed
    pub max_width_for_line: Cell<Option<MaxWidthForLine>>,
    pub indent: Cell<usize>,
    pub newline_budget: Cell<Option<usize>>,
}

impl Default for Constraints {
    fn default() -> Self {
        Constraints::new(Config::default().max_width)
    }
}

impl Constraints {
    pub fn new(max_width: u32) -> Constraints {
        Constraints {
            indent: Cell::new(0),
            max_width: Cell::new(Some(max_width)),
            max_width_for_line: Cell::new(None),
            newline_budget: Cell::new(None),
            single_line: Cell::new(false),
        }
    }

    pub fn set(&self, other: &Constraints) {
        let Constraints {
            indent,
            max_width,
            max_width_for_line: max_width_first_line,
            newline_budget,
            single_line,
        } = other;
        self.indent.set(indent.get());
        self.max_width.set(max_width.get());
        self.max_width_for_line.set(max_width_first_line.get());
        self.newline_budget.set(newline_budget.get());
        self.single_line.set(single_line.get());
    }

    pub fn increment_indent(&self) {
        self.indent.set(self.indent.get() + INDENT_WIDTH);
    }

    pub fn decrement_indent(&self) {
        self.indent.set(self.indent.get() - INDENT_WIDTH);
    }

    pub fn add_max_width(&self, len: u32) {
        if let Some(max_width) = self.max_width.get() {
            self.max_width.set(Some(max_width + len));
        }
    }

    pub fn sub_max_width(&self, len: u32) -> Result<(), WidthLimitExceededError> {
        if let Some(max_width) = self.max_width.get() {
            self.max_width.set(Some(
                max_width.checked_sub(len).ok_or(WidthLimitExceededError)?,
            ));
        }
        Ok(())
    }

    pub fn with_no_max_width(&self, f: impl FnOnce() -> FormatResult) -> FormatResult {
        let max_width_prev = self.max_width.replace(None);
        let result = f();
        self.max_width.set(max_width_prev);
        result
    }
}
