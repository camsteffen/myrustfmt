use crate::ast_formatter::AstFormatter;
use crate::constraints::{Constraints, VStruct, VStructSet, WidthLimit};
use crate::error::{FormatErrorKind, FormatResult};
use crate::num::HSize;
use crate::util::cell_ext::CellExt;
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
    pub fn with_replace_width_limit<T>(&self, width_limit: Option<WidthLimit>, scope: impl FnOnce() -> T) -> T;

    // vertical structures
    pub fn allow_vstructs(&self, values: impl Into<VStructSet>, scope: impl FnOnce() -> FormatResult) -> FormatResult;
    pub fn disallow_vstructs(&self, values: impl Into<VStructSet>, scope: impl FnOnce() -> FormatResult) -> FormatResult;
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

    pub fn with_single_line_if<T>(
        &self,
        condition: bool,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if !condition {
            return scope();
        }
        self.with_single_line(scope)
    }

    pub fn with_width_limit<T>(
        &self,
        width_limit: HSize,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let end_col = self.out.col() + width_limit;
        self.with_width_limit_end(end_col, scope)
    }

    pub fn with_width_limit_opt<T>(
        &self,
        width_limit: Option<HSize>,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        match width_limit {
            None => scope(),
            Some(width_limit) => self.with_width_limit(width_limit, scope),
        }
    }

    pub fn with_width_limit_end<T>(
        &self,
        end_col: HSize,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let line = self.out.line();
        if self.out.col() > end_col {
            return Err(FormatErrorKind::WidthLimitExceeded.into());
        }
        let end_col = NonZero::new(end_col).expect("width limit end should not be zero");
        let limit = WidthLimit { end_col, line };
        self.constraints().with_width_limit(limit, scope)
    }

    pub fn with_width_limit_end_opt<T>(
        &self,
        end_col: Option<HSize>,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(end_col) = end_col else {
            return scope();
        };
        self.with_width_limit_end(end_col, scope)
    }
}
