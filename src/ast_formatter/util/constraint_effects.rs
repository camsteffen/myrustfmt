use crate::ast_formatter::AstFormatter;
use crate::constraints::{MaxWidthForLine, MultiLineShape};
use crate::error::{FormatResult, WidthLimitExceededError};

impl AstFormatter {
    pub fn with_single_line<T>(&self, format: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        self.constraints()
            .with_multi_line_shape_replaced(MultiLineShape::SingleLine, format)
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
        self.constraints().with_max_width(Some(max_width), format)
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
}
