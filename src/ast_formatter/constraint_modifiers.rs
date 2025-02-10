use crate::ast_formatter::AstFormatter;
use crate::constraints::{Constraints, MaxWidthForLine, MultiLineConstraint};
use crate::error::{FormatResult, WidthLimitExceededError};
use crate::util::cell_ext::CellExt;

pub const INDENT_WIDTH: u32 = 4;

impl AstFormatter {
    pub(super) fn constraints(&self) -> &Constraints {
        self.out.constraints()
    }

    pub fn indented<T>(&self, f: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        let indent = self.constraints().indent.get() + INDENT_WIDTH;
        self.constraints()
            .indent
            .with_replaced(indent, || match self.constraints().multi_line.get() {
                MultiLineConstraint::SingleLine | MultiLineConstraint::MultiLine => f(),
                _ => {
                    self.constraints()
                        .multi_line
                        .with_replaced(MultiLineConstraint::MultiLine, f)
                }
            })
    }

    pub fn indented_optional(
        &self,
        should_indent: bool,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        if !should_indent {
            return f();
        }
        self.indented(f)
    }

    pub fn with_single_line<T>(&self, f: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        assert!(
            self.constraints().has_open_checkpoints(),
            "single line constraint applied with no fallback"
        );
        self.constraints()
            .multi_line
            .with_replaced(MultiLineConstraint::SingleLine, f)
    }

    pub fn with_single_line_opt<T>(
        &self,
        apply: bool,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if apply { self.with_single_line(f) } else { f() }
    }

    /** Enforces a max number of characters until a newline is printed */
    pub fn with_width_limit_first_line<T>(&self, width_limit: u32, f: impl FnOnce() -> T) -> T {
        let line = self.out.line();
        let max_width = self.out.last_line_len() + width_limit;
        if self.constraints().max_width.get().is_some_and(|mw| {
            max_width >= mw
        })
            || self.constraints().max_width_for_line.get().is_some_and(
                |m| m.line == line && m.max_width <= max_width,
            )
        {
            return f();
        }
        self.constraints()
            .max_width_for_line
            .with_replaced(Some(MaxWidthForLine { line, max_width }), f)
    }

    pub fn with_width_limit_first_line_opt<T>(
        &self,
        width_limit: Option<u32>,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        match width_limit {
            None => f(),
            Some(width_limit) => self.with_width_limit_first_line(width_limit, f),
        }
    }

    pub fn with_width_limit_from_start<T>(
        &self,
        line_start_pos: u32,
        width_limit: u32,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(remaining) = width_limit
            .checked_sub(self.out.last_line_len() - line_start_pos) else {
            return Err(WidthLimitExceededError.into());
        };
        self.with_width_limit(remaining, f)
    }

    pub fn with_width_limit_from_start_opt<T>(
        &self,
        line_start_pos: u32,
        width_limit: Option<u32>,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(width_limit) = width_limit else {
            return f();
        };
        self.with_width_limit_from_start(
            line_start_pos,
            width_limit,
            f,
        )
    }

    pub fn with_width_limit_from_start_first_line<T>(
        &self,
        line_start_pos: u32,
        width_limit: u32,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(remaining) = width_limit
            .checked_sub(self.out.last_line_len() - line_start_pos) else {
            return Err(WidthLimitExceededError.into());
        };
        self.with_width_limit_first_line(remaining, f)
    }

    pub fn with_width_limit_from_start_first_line_opt<T>(
        &self,
        line_start_pos: u32,
        width_limit: Option<u32>,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(width_limit) = width_limit else {
            return f();
        };
        self.with_width_limit_from_start_first_line(
            line_start_pos,
            width_limit,
            f,
        )
    }

    pub fn with_width_limit<T>(
        &self,
        width_limit: u32,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let max_width = self.out.last_line_len() + width_limit;
        if self.constraints().max_width.get().is_some_and(|m| {
            m <= max_width
        }) {
            f()
        } else {
            self.constraints()
                .max_width
                .with_replaced(Some(max_width), f)
        }
    }

    pub fn with_width_limit_opt(
        &self,
        width_limit: Option<u32>,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        match width_limit {
            None => f(),
            Some(width_limit) => self.with_width_limit(width_limit, f),
        }
    }

    pub fn with_single_line_and_width_limit(
        &self,
        width_limit: u32,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        self.with_single_line(|| {
            self.with_width_limit(width_limit, f)
        })
    }
}
