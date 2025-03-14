use std::num::NonZero;
use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::constraints::{VerticalShape, WidthLimit};
use crate::error::{FormatResult, WidthLimitExceededError};
use crate::num::HPos;

macro_rules! delegate_to_constraints {
    ($($vis:vis fn $name:ident $(<$gen:tt>)?(&self $(, $arg:ident: $ty:ty)* $(,)?) $(-> $ret_ty:ty)? ;)*) => {
        impl AstFormatter {
            $($vis fn $name $(<$gen>)? (&self $(, $arg: $ty)*) $(-> $ret_ty)? {
                self.constraints().$name($($arg),*)
            })*
        }
    }
}

delegate_to_constraints! {
    // max width
    pub fn with_replace_max_width<T>(&self, max_width: HPos, scope: impl FnOnce() -> T) -> T;
    
    // width limit
    pub fn width_limit(&self) -> Option<WidthLimit>;
    pub fn with_replace_width_limit<T>(&self, width_limit: Option<WidthLimit>, scope: impl FnOnce() -> T) -> T;
    
    // vertical shape
    pub fn vertical_shape(&self) -> VerticalShape;
    pub fn with_single_line<T>(&self, format: impl FnOnce() -> T) -> T;
    pub fn with_single_line_opt<T>(&self, apply: bool, scope: impl FnOnce() -> FormatResult<T>) -> FormatResult<T>;
    pub fn with_replace_vertical_shape<T>(&self, vertical_shape: VerticalShape, scope: impl FnOnce() -> T) -> T;
    pub fn with_vertical_shape_min<T>(&self, shape: VerticalShape, scope: impl FnOnce() -> T) -> T;
    pub fn has_vertical_shape<T>(&self, shape: VerticalShape, scope: impl FnOnce() -> FormatResult<T>) -> FormatResult<T>;
    pub fn has_vertical_shape_unless<T>(&self, shape: VerticalShape, condition: bool, scope: impl FnOnce() -> FormatResult<T>) -> FormatResult<T>;
}

impl AstFormatter {
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
    /// first line without short-circuiting. See also `VerticalShape`.)
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
        let result = self.with_single_line(|| self.with_replace_max_width(max_width, scope));
        let used_extra_width = self.out.last_line_len() > max_width_prev;
        (used_extra_width, result)
    }
}
