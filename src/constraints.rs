use crate::config::Config;
use crate::error::FormatResult;
use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaxWidthForLine {
    pub line: u32,
    pub max_width: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Constraints {
    pub max_width: Cell<Option<u32>>,
    /// Used to set the max width for the current line, so it no longer applies after a newline
    /// character is printed
    pub max_width_for_line: Cell<Option<MaxWidthForLine>>,
    /// If true, no newline characters allowed
    pub single_line: Cell<bool>,
    // todo is this a constraint?
    pub indent: Cell<usize>,
    /// When true, we say the margin doesn't like to be touched by the expression in question.
    /// If an expression is touching the margin too much, we wrap it with a block to push it away.
    /// More specifically, the expression should only touch the margin in its first and last lines.
    ///
    /// In the example below, the chain expression is not indented, and so is "touching the margin"
    /// for every item in the chain. This makes it harder to scan the arm patterns of the match.
    /// Therefore, match arm bodies are formatted with `touchy_margin` enabled. In the example
    /// below, this would cause the chain to be wrapped with a block.
    ///
    /// ```
    /// match x {
    ///     Some(pattern) => a({
    ///         todo!()
    ///     })
    ///     .multi_line
    ///     .chain,
    ///     _ => {}
    /// }
    /// ```
    pub touchy_margin: Cell<bool>,
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
            single_line: Cell::new(false),
            touchy_margin: Cell::new(false),
        }
    }

    pub fn set(&self, other: &Constraints) {
        let Constraints {
            indent,
            max_width,
            max_width_for_line: max_width_first_line,
            single_line,
            touchy_margin,
        } = other;
        self.indent.set(indent.get());
        self.max_width.set(max_width.get());
        self.max_width_for_line.set(max_width_first_line.get());
        self.single_line.set(single_line.get());
        self.touchy_margin.set(touchy_margin.get());
    }

    pub fn with_no_max_width(&self, f: impl FnOnce() -> FormatResult) -> FormatResult {
        let max_width_prev = self.max_width.replace(None);
        let result = f();
        self.max_width.set(max_width_prev);
        result
    }
}
