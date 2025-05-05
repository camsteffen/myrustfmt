use crate::ast_formatter::AstFormatter;
use crate::constraints::{Constraints, VerticalShape, WidthLimit};
use crate::error::{ConstraintErrorKind, FormatResult, WidthLimitExceededError};
use crate::num::HPos;
use std::num::NonZero;

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
    pub fn with_replace_vertical_shape<T>(&self, vertical_shape: VerticalShape, scope: impl FnOnce() -> T) -> T;
    pub fn has_vertical_shape<T>(&self, shape: VerticalShape, scope: impl FnOnce() -> FormatResult<T>) -> FormatResult<T>;
    pub fn has_vertical_shape_if<T>(&self, condition: bool, shape: VerticalShape, scope: impl FnOnce() -> FormatResult<T>) -> FormatResult<T>;
}

impl AstFormatter {
    pub fn constraints(&self) -> &Constraints {
        self.out.constraints()
    }

    pub fn with_single_line<T>(&self, format: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        match self
            .constraints()
            .with_replace_vertical_shape(VerticalShape::SingleLine, format)
        {
            Err(mut e) if e.kind == ConstraintErrorKind::NewlineNotAllowed => {
                e.kind = ConstraintErrorKind::NextStrategy;
                Err(e)
            }
            result => result,
        }
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

    /// Requires the given scope to conform to the given VerticalShape
    pub fn with_vertical_shape_min<T>(
        &self,
        shape: VerticalShape,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if self.vertical_shape() <= shape {
            return scope();
        }
        match self.constraints().with_replace_vertical_shape(shape, scope) {
            Err(mut e) if e.kind == ConstraintErrorKind::NewlineNotAllowed => {
                e.kind = ConstraintErrorKind::NextStrategy;
                Err(e)
            }
            result => result,
        }
    }

    pub fn with_width_limit<T>(
        &self,
        width_limit: HPos,
        format: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let end = NonZero::new(self.out.last_line_len() + width_limit)
            .expect("width limit should not end at column zero");
        self.constraints()
            .with_width_limit(WidthLimit::SingleLine { end }, format)
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
}
