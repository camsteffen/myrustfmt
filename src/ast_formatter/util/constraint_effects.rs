use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::constraints::{MaxWidthForLine, MultiLineShape};
use crate::error::{FormatResult, WidthLimitExceededError};

impl AstFormatter {
    pub fn with_single_line<T>(&self, format: impl FnOnce() -> T) -> T {
        self.constraints()
            .with_multi_line_shape_replaced(MultiLineShape::SingleLine, format)
    }

    pub fn with_single_line_opt<T>(
        &self,
        apply: bool,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if !apply {
            return scope();
        }
        self.with_single_line(scope)
    }

    pub fn with_width_limit<T>(
        &self,
        width_limit: u32,
        format: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let max_width = self.out.last_line_len() + width_limit;
        if self.out.current_max_width().is_some_and(|m| m <= max_width) {
            return format();
        }
        self.out
            .with_enforce_max_width(|| self.constraints().with_max_width(Some(max_width), format))
    }

    pub fn with_width_limit_first_line<T>(
        &self,
        width_limit: u32,
        format: impl FnOnce() -> T,
    ) -> T {
        let line = self.out.line();
        let max_width = self.out.last_line_len() + width_limit;
        if self
            .out
            .current_max_width()
            .is_some_and(|mw| max_width >= mw)
        {
            return format();
        }
        self.constraints()
            .with_max_width_for_line(Some(MaxWidthForLine { line, max_width }), format)
    }

    pub fn with_width_limit_first_line_opt<T>(
        &self,
        width_limit: Option<u32>,
        format: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        match width_limit {
            None => format(),
            Some(width_limit) => self.with_width_limit_first_line(width_limit, format),
        }
    }

    pub fn with_width_limit_from_start<T>(
        &self,
        line_start_pos: u32,
        width_limit: u32,
        format: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(remaining) = width_limit.checked_sub(self.out.last_line_len() - line_start_pos)
        else {
            return Err(WidthLimitExceededError.into());
        };
        self.with_width_limit(remaining, format)
    }

    pub fn with_width_limit_from_start_first_line<T>(
        &self,
        line_start_pos: u32,
        width_limit: u32,
        format: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(remaining) = width_limit.checked_sub(self.out.last_line_len() - line_start_pos)
        else {
            return Err(WidthLimitExceededError.into());
        };
        self.with_width_limit_first_line(remaining, format)
    }

    pub fn with_width_limit_from_start_first_line_opt<T>(
        &self,
        line_start_pos: u32,
        width_limit: Option<u32>,
        format: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(width_limit) = width_limit else {
            return format();
        };
        self.with_width_limit_from_start_first_line(line_start_pos, width_limit, format)
    }

    /// 1. Adds a single-line constraint.
    /// 2. Adds to the max width to simulate having just wrapped to the next line with an added
    ///    indent.
    ///
    /// Returns a bool to indicate if the extra width was used in the resulting output.
    /// If the bool is true, the result cannot be used, but this may indicate that you should wrap
    /// and indent, or add a block.
    ///
    /// If formatting fails with a newline-not-allowed error, it is still useful to observe the
    /// boolean to know whether the first line of code (the code emitted leading up to the error)
    /// used the extra width. (This does assume that downstream formatting will emit all of the
    /// first line without short-circuiting. See also `MultiLineShape`.)
    ///
    /// When the extra width is used, this means one of two things: either the extra width allowed
    /// for a different formatting strategy with more code on the first line, or the extra width was
    /// strictly required to fit the code at all. This function is useful when these two cases are
    /// handled in the same way.
    // todo return an enum? usage sites seem to follow the same decision tree
    pub fn simulate_wrap_indent_first_line<T>(&self, scope: impl FnOnce() -> T) -> (bool, T) {
        let start = self.out.last_line_len();
        // the starting position if we wrapped to the next line and indented
        let next_line_start = self.out.indent.get() + INDENT_WIDTH;
        let extra_width = start.checked_sub(next_line_start);
        let max_width_prev = self.out.current_max_width();
        let max_width = max_width_prev.map(|w| w + extra_width.unwrap_or(0));
        let result =
            self.with_single_line(|| self.constraints().with_global_max_width(max_width, scope));
        let used_extra_width = max_width_prev.is_some_and(|w| self.out.last_line_len() > w);
        (used_extra_width, result)
    }
}
