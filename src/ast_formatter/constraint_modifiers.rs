use crate::ast_formatter::AstFormatter;
use crate::constraints::Constraints;
use crate::error::FormatResult;

impl AstFormatter {
    fn constraints(&self) -> &Constraints {
        self.out.constraints()
    }

    pub fn indented(&self, f: impl FnOnce() -> FormatResult) -> FormatResult {
        self.constraints().increment_indent();
        let result = f();
        self.constraints().decrement_indent();
        result
    }

    pub fn indented_optional(&self, should_indent: bool, f: impl FnOnce() -> FormatResult) -> FormatResult {
        if !should_indent {
            return f();
        }
        self.constraints().increment_indent();
        let result = f();
        self.constraints().decrement_indent();
        result
    }

    pub fn with_no_overflow(&self, f: impl FnOnce() -> FormatResult) -> FormatResult {
        let allow_overflow_prev = self.allow_overflow.replace(false);
        let result = f();
        self.allow_overflow.set(allow_overflow_prev);
        result
    }

    pub fn with_single_line(&self, f: impl FnOnce() -> FormatResult) -> FormatResult {
        let single_line_prev = self.constraints().single_line.replace(true);
        let result = f();
        self.constraints().single_line.set(single_line_prev);
        result
    }

    pub fn with_single_line_optional(&self, apply: bool, f: impl FnOnce() -> FormatResult) -> FormatResult {
        if !apply {
            return f();
        }
        self.with_single_line(f)
    }

    pub fn with_not_single_line(&self, f: impl FnOnce() -> FormatResult) -> FormatResult {
        let single_line_prev = self.constraints().single_line.replace(false);
        let result = f();
        self.constraints().single_line.set(single_line_prev);
        result
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

    /** Enforces a max number of characters until a newline is printed */
    pub fn with_width_limit_first_line(
        &self,
        width_limit: usize,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        let max_width = self.out.last_line_len() + width_limit;
        if self
            .constraints()
            .max_width_first_line
            .get()
            .is_some_and(|m| m <= max_width)
        {
            return f();
        }
        let max_width_prev = self
            .constraints()
            .max_width_first_line
            .replace(Some(max_width));
        let result = f();
        self.constraints().max_width_first_line.set(max_width_prev);
        result
    }

    pub fn with_width_limit_first_line_opt(
        &self,
        width_limit: Option<usize>,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        match width_limit {
            None => f(),
            Some(width_limit) => self.with_width_limit_first_line(width_limit, f),
        }
    }

    pub fn with_width_limit(
        &self,
        width_limit: usize,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
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
