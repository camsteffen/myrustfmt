use std::num::NonZero;
use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::constraints::{MultiLineShape, WidthLimit};
use crate::error::{FormatResult, WidthLimitExceededError};
use crate::num::HPos;
use crate::util::cell_ext::CellExt;

impl AstFormatter {
    pub fn with_single_line<T>(&self, format: impl FnOnce() -> T) -> T {
        self.constraints()
            .multi_line
            .with_replaced(MultiLineShape::SingleLine, format)
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
        width_limit: HPos,
        format: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let end = NonZero::new(self.out.last_line_len() + width_limit)
            .unwrap();
        // todo enforce max width here?
        self.out.with_enforce_max_width(|| {
            self.constraints()
                .with_width_limit(WidthLimit::SingleLine { end }, format)
        })
    }

    pub fn with_width_limit_first_line<T>(
        &self,
        width_limit: HPos,
        format: impl FnOnce() -> T,
    ) -> T {
        let line = self.out.line();
        let end = NonZero::new(self.out.last_line_len() + width_limit)
            .unwrap();
        self.constraints()
            .with_width_limit(WidthLimit::FirstLine { end, line }, format)
    }

    pub fn with_width_limit_first_line_opt<T>(
        &self,
        width_limit: Option<HPos>,
        format: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        match width_limit {
            None => format(),
            Some(width_limit) => self.with_width_limit_first_line(width_limit, format),
        }
    }

    pub fn with_width_limit_from_start<T>(
        &self,
        line_start_pos: HPos,
        width_limit: HPos,
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
        line_start_pos: HPos,
        width_limit: HPos,
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
        line_start_pos: HPos,
        width_limit: Option<HPos>,
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
        let Some(extra_width) = start.checked_sub(next_line_start).filter(|&w| w > 0) else {
            let result = self.with_single_line(scope);
            return (false, result);
        };
        let max_width_prev = self.out.current_max_width();
        let max_width = max_width_prev.saturating_add(extra_width);
        let result =
            self.with_single_line(|| self.constraints().max_width.with_replaced(max_width, scope));
        let used_extra_width = self.out.last_line_len() > max_width_prev;
        (used_extra_width, result)
    }
}
