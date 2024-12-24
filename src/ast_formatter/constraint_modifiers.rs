use std::backtrace::Backtrace;
use std::rc::Rc;
use crate::ast_formatter::AstFormatter;
use crate::constraints::{Constraints, MaxWidthForLine};
use crate::error::{FormatResult, WidthLimitExceededError};
use tracing::info;

impl AstFormatter {
    fn constraints(&self) -> &Constraints {
        self.out.constraints()
    }

    pub fn indented<T>(&self, f: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        self.constraints().increment_indent();
        let result = f();
        self.constraints().decrement_indent();
        result
    }

    pub fn indented_optional(
        &self,
        should_indent: bool,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        if !should_indent {
            return f();
        }
        self.constraints().increment_indent();
        let result = f();
        self.constraints().decrement_indent();
        result
    }

    pub fn with_no_multiline_overflow(&self, f: impl FnOnce() -> FormatResult) -> FormatResult {
        let allow_overflow_prev = self.allow_multiline_overflow.replace(false);
        let result = f();
        self.allow_multiline_overflow.set(allow_overflow_prev);
        result
    }

    pub fn with_no_multiline_overflow_optional(&self, apply: bool, f: impl FnOnce() -> FormatResult) -> FormatResult {
        if !apply {
            return f();
        }
        self.with_no_multiline_overflow(f)
    }

    pub fn with_single_line<T>(&self, f: impl FnOnce() -> T) -> T {
        self.with_replace_single_line(true, f)
    }

    pub fn with_replace_single_line<T>(&self, value: bool, f: impl FnOnce() -> T) -> T {
        let single_line_prev = self.constraints().single_line.replace(value);
        let bp = self.constraints().single_line_backtrace.replace(Some(Rc::new(Backtrace::capture())));
        let result = f();
        self.constraints().single_line.set(single_line_prev);
        self.constraints().single_line_backtrace.replace(bp);
        result
    }

    pub fn with_single_line_optional(
        &self,
        apply: bool,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        if !apply {
            return f();
        }
        self.with_single_line(f)
    }

    pub fn with_not_single_line(&self, f: impl FnOnce() -> FormatResult) -> FormatResult {
        self.with_replace_single_line(false, f)
    }

    pub fn with_do_overflow(&self, f: impl Fn() -> FormatResult) -> FormatResult {
        if self.config().rustfmt_quirks {
            self.fallback(&f)
                .next(|| {
                    info!("{:?}", self.constraints());
                    self.with_reduce_max_width_for_line(2, || self.with_not_single_line(f))
                })
                .result()
        } else {
            self.with_not_single_line(f)
        }
    }

    pub fn with_height_limit(
        &self,
        height: usize,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        let newlines = height - 1;
        if self
            .constraints()
            .newline_budget
            .get()
            .is_some_and(|h| h <= newlines)
        {
            return f();
        }
        let height_budget_prev = self.constraints().newline_budget.replace(Some(newlines));
        let result = f();
        self.constraints().newline_budget.set(height_budget_prev);
        result
    }

    pub fn with_dimensions(
        &self,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult<(usize, usize)> {
        let len = self.out.len();
        let line = self.out.line();
        f()?;
        Ok((self.out.len() - len, self.out.line() - line))
    }

    pub fn with_reduce_width_limit(
        &self,
        amount: usize,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        let Some(current) = self.constraints().max_width.get() else {
            return f();
        };
        let Some(new_limit) = current.checked_sub(amount) else {
            return Err(WidthLimitExceededError.into());
        };
        self.with_width_limit(new_limit, f)
    }

    pub fn with_reduce_max_width_for_line(
        &self,
        amount: usize,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        let Some(max_width_for_line) = self.constraints().max_width_for_line.get() else {
            return f();
        };
        if max_width_for_line.line != self.out.line() {
            return f();
        }
        let Some(new_max_width) = max_width_for_line.max_width.checked_sub(amount) else {
            return Err(WidthLimitExceededError.into());
        };
        self.with_set_max_width_for_line(new_max_width, f)
    }

    fn with_set_max_width_for_line<T>(&self, max_width: usize, f: impl FnOnce() -> T) -> T {
        let line = self.out.line();
        let max_width_prev = self
            .constraints()
            .max_width_for_line
            .replace(Some(MaxWidthForLine { line, max_width }));
        let result = f();
        self.constraints().max_width_for_line.set(max_width_prev);
        result
    }

    /** Enforces a max number of characters until a newline is printed */
    pub fn with_width_limit_first_line<T>(&self, width_limit: usize, f: impl FnOnce() -> T) -> T {
        let line = self.out.line();
        let max_width = self.out.last_line_len() + width_limit;
        if self
            .constraints()
            .max_width_for_line
            .get()
            .is_some_and(|m| m.line == line && m.max_width <= max_width)
        {
            return f();
        }
        self.with_set_max_width_for_line(max_width, f)
    }

    pub fn with_width_limit_first_line_opt<T>(
        &self,
        width_limit: Option<usize>,
        f: impl FnOnce() -> T,
    ) -> T {
        match width_limit {
            None => f(),
            Some(width_limit) => self.with_width_limit_first_line(width_limit, f),
        }
    }

    pub fn with_width_limit_from_start_first_line_opt<T>(
        &self,
        line_start_pos: usize,
        width_limit: Option<usize>,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(width_limit) = width_limit else {
            return f();
        };
        let Some(remaining) = width_limit.checked_sub(self.out.last_line_len() - line_start_pos)
        else {
            return Err(WidthLimitExceededError.into());
        };
        self.with_width_limit_first_line(remaining, f)
    }

    pub fn with_width_limit<T>(
        &self,
        width_limit: usize,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let max_width = self.out.last_line_len() + width_limit;
        if self
            .constraints()
            .max_width
            .get()
            .is_some_and(|m| m <= max_width)
        {
            return f();
        }
        let max_width_prev = self.constraints().max_width.replace(Some(max_width));
        let result = f();
        self.constraints().max_width.set(max_width_prev);
        result
    }

    pub fn with_width_limit_opt(
        &self,
        width_limit: Option<usize>,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        match width_limit {
            None => f(),
            Some(width_limit) => self.with_width_limit(width_limit, f),
        }
    }

    pub fn with_width_limit_single_line(
        &self,
        width_limit: usize,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        self.with_width_limit(width_limit, || self.with_single_line(f))
    }
}
