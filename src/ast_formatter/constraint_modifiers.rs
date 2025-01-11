use crate::ast_formatter::AstFormatter;
use crate::constraints::{Constraints, MaxWidthForLine};
use crate::error::{FormatResult, WidthLimitExceededError};

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

    pub fn with_single_line<T>(&self, f: impl FnOnce() -> T) -> T {
        let single_line_prev = self.constraints().single_line.replace(true);
        let result = f();
        self.constraints().single_line.set(single_line_prev);
        result
    }

    pub fn with_dimensions(&self, f: impl FnOnce() -> FormatResult) -> FormatResult<(usize, u32)> {
        let len = self.out.len();
        let line = self.out.line();
        f()?;
        Ok((self.out.len() - len, self.out.line() - line))
    }

    pub fn with_reduce_max_width_for_line(
        &self,
        amount: u32,
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

    fn with_set_max_width_for_line<T>(&self, max_width: u32, f: impl FnOnce() -> T) -> T {
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
    pub fn with_width_limit_first_line<T>(&self, width_limit: u32, f: impl FnOnce() -> T) -> T {
        let line = self.out.line();
        let max_width = self.out.last_line_len() as u32 + width_limit;
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
        width_limit: Option<u32>,
        f: impl FnOnce() -> T,
    ) -> T {
        match width_limit {
            None => f(),
            Some(width_limit) => self.with_width_limit_first_line(width_limit, f),
        }
    }

    pub fn with_width_limit_from_start<T>(
        &self,
        line_start_pos: usize,
        width_limit: u32,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(remaining) =
            width_limit.checked_sub((self.out.last_line_len() - line_start_pos) as u32)
        else {
            return Err(WidthLimitExceededError.into());
        };
        self.with_width_limit(remaining, f)
    }

    pub fn with_width_limit_from_start_opt<T>(
        &self,
        line_start_pos: usize,
        width_limit: Option<u32>,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(width_limit) = width_limit else {
            return f();
        };
        self.with_width_limit_from_start(line_start_pos, width_limit, f)
    }

    pub fn with_width_limit_from_start_first_line<T>(
        &self,
        line_start_pos: usize,
        width_limit: u32,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(remaining) =
            width_limit.checked_sub((self.out.last_line_len() - line_start_pos) as u32)
        else {
            return Err(WidthLimitExceededError.into());
        };
        self.with_width_limit_first_line(remaining, f)
    }

    pub fn with_width_limit_from_start_first_line_opt<T>(
        &self,
        line_start_pos: usize,
        width_limit: Option<u32>,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(width_limit) = width_limit else {
            return f();
        };
        self.with_width_limit_from_start_first_line(line_start_pos, width_limit, f)
    }

    pub fn with_width_limit<T>(
        &self,
        width_limit: u32,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let max_width = self.out.last_line_len() as u32 + width_limit;
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
        width_limit: Option<u32>,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        match width_limit {
            None => f(),
            Some(width_limit) => self.with_width_limit(width_limit, f),
        }
    }

    pub fn with_width_limit_single_line(
        &self,
        width_limit: u32,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        self.with_width_limit(width_limit, || self.with_single_line(f))
    }
}
