use crate::ast_formatter::AstFormatter;
use crate::constraints::{Constraints, VStruct, WidthLimit};
use crate::error::{FormatResult, WidthLimitExceededError};
use crate::num::HSize;
use crate::util::cell_ext::CellExt;
use enumset::EnumSet;
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
    pub fn with_replace_max_width<T>(&self, max_width: HSize, scope: impl FnOnce() -> T) -> T;

    // width limit
    pub fn width_limit(&self) -> Option<WidthLimit>;
    pub fn with_replace_width_limit<T>(&self, width_limit: Option<WidthLimit>, scope: impl FnOnce() -> T) -> T;

    // shape
    pub fn disallow_vstructs(&self, values: impl Into<EnumSet<VStruct>>, scope: impl FnOnce() -> FormatResult) -> FormatResult;
    pub fn has_vstruct<T>(&self, vstruct: VStruct, scope: impl FnOnce() -> FormatResult<T>) -> FormatResult<T>;
}

impl AstFormatter {
    pub fn constraints(&self) -> &Constraints {
        self.out.constraints()
    }

    pub fn has_vstruct_if(
        &self,
        condition: bool,
        vstruct: VStruct,
        scope: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        if condition {
            self.has_vstruct(vstruct, scope)
        } else {
            scope()
        }
    }

    pub fn with_single_line<T>(&self, scope: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        self.constraints()
            .single_line
            .with_replaced(true, || self.out.with_recover_width(scope))
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
        width_limit: HSize,
        format: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        // todo assert there is a checkpoint since width limits enable max width enforcement
        let end = NonZero::new(self.out.col() + width_limit)
            .expect("width limit should not end at column zero");
        self.constraints()
            .with_width_limit(WidthLimit::SingleLine { end }, format)
    }

    pub fn with_width_limit_first_line<T>(
        &self,
        width_limit: HSize,
        format: impl FnOnce() -> T,
    ) -> T {
        let (line, col) = self.out.line_col();
        let end = NonZero::new(col + width_limit).unwrap();
        self.constraints()
            .with_width_limit(WidthLimit::FirstLine { end, line }, format)
    }

    pub fn with_width_limit_first_line_opt<T>(
        &self,
        width_limit: Option<HSize>,
        format: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        match width_limit {
            None => format(),
            Some(width_limit) => self.with_width_limit_first_line(width_limit, format),
        }
    }

    pub fn with_width_limit_from_start<T>(
        &self,
        start_col: HSize,
        width_limit: HSize,
        format: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(remaining) = width_limit.checked_sub(self.out.col() - start_col) else {
            return Err(WidthLimitExceededError.into());
        };
        self.with_width_limit(remaining, format)
    }

    pub fn with_width_limit_from_start_first_line<T>(
        &self,
        start_col: HSize,
        width_limit: HSize,
        format: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(remaining) = width_limit.checked_sub(self.out.col() - start_col) else {
            return Err(WidthLimitExceededError.into());
        };
        self.with_width_limit_first_line(remaining, format)
    }

    pub fn with_width_limit_from_start_first_line_opt<T>(
        &self,
        start_col: HSize,
        width_limit: Option<HSize>,
        format: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(width_limit) = width_limit else {
            return format();
        };
        self.with_width_limit_from_start_first_line(start_col, width_limit, format)
    }
}
