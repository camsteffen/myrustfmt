use crate::config::Config;
use crate::error::{FormatResult, WidthLimitExceededError};
use std::backtrace::Backtrace;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub const INDENT_WIDTH: usize = 4;

#[derive(Clone, Copy, Debug)]
pub struct MaxWidthForLine {
    pub line: usize,
    pub max_width: usize,
}

#[derive(Clone, Debug)]
pub struct Constraints {
    pub single_line: Cell<bool>,
    pub single_line_backtrace: RefCell<Option<Rc<Backtrace>>>,
    pub max_width: Cell<Option<usize>>,
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
    pub fn new(max_width: usize) -> Constraints {
        Constraints {
            indent: Cell::new(0),
            max_width: Cell::new(Some(max_width)),
            max_width_for_line: Cell::new(None),
            newline_budget: Cell::new(None),
            single_line: Cell::new(false),
            single_line_backtrace: RefCell::new(None),
        }
    }

    pub fn set(&self, other: &Constraints) {
        let Constraints {
            indent,
            max_width,
            max_width_for_line: max_width_first_line,
            newline_budget,
            single_line,
            single_line_backtrace,
        } = other;
        self.indent.set(indent.get());
        self.max_width.set(max_width.get());
        self.max_width_for_line.set(max_width_first_line.get());
        self.newline_budget.set(newline_budget.get());
        self.single_line.set(single_line.get());
        self.single_line_backtrace
            .replace(single_line_backtrace.borrow().clone());
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

    pub fn sub_max_width(&self, len: usize) -> Result<(), WidthLimitExceededError> {
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
