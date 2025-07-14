use crate::Recover;
use crate::ast_formatter::AstFormatter;
use crate::constraints::{Constraints, VStruct, VStructSet, WidthLimit};
use crate::error::{FormatError, FormatErrorKind, FormatResult};
use crate::num::HSize;
use crate::util::cell_ext::CellExt;
use crate::util::drop::Guard;
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
    pub fn err(&self, kind: FormatErrorKind) -> FormatError;
    pub fn disallow_vstructs(&self, values: impl Into<VStructSet>, recover: &Recover, scope: impl FnOnce() -> FormatResult) -> FormatResult;
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

    pub fn recover_width_guard(&self) -> impl Guard {
        self.constraints().recover_width.replace_guard(Some(
            self.out.line(),
        ))
    }

    pub fn single_line_guard(&self) -> impl Guard {
        (
            self.constraints().single_line.replace_guard(true),
            self.recover_width_guard(),
        )
    }

    pub fn width_limit_end_col(&self) -> Option<NonZero<HSize>> {
        self.constraints().width_limit_end_col(self.out.line())
    }

    pub fn width_limit_guard(&self, width_limit: HSize) -> FormatResult<Option<impl Guard>> {
        let end_col = self.out.col().saturating_add(width_limit);
        self.width_limit_end_guard(end_col)
    }

    pub fn width_limit_opt_guard(
        &self,
        width_limit: Option<HSize>,
    ) -> FormatResult<Option<impl Guard>> {
        let Some(width_limit) = width_limit else {
            return Ok(None);
        };
        self.width_limit_guard(width_limit)
    }

    pub fn width_limit_end_guard(&self, end_col: HSize) -> FormatResult<Option<impl Guard>> {
        let line = self.out.line();
        if self.out.col() > end_col {
            return Err(self.err(FormatErrorKind::WidthLimitExceeded));
        }
        let end_col = NonZero::new(end_col).expect("width limit end should not be zero");
        let limit = WidthLimit {
            end_col,
            line,
            simulate: None,
        };
        Ok(self.constraints().width_limit_guard(limit))
    }

    pub fn width_limit_end_opt_guard(
        &self,
        end_col: Option<HSize>,
    ) -> FormatResult<Option<impl Guard>> {
        let Some(end_col) = end_col else {
            return Ok(None);
        };
        self.width_limit_end_guard(end_col)
    }
}
