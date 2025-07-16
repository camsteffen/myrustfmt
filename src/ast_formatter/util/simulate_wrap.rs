use crate::ast_formatter::AstFormatter;
use crate::constraints::{WidthLimit, WidthLimitSimulate};
use crate::error::{FormatErrorKind, FormatResult};
use crate::num::HSize;
use crate::util::cell_ext::CellExt;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimulateWrapResult {
    /// The result may be used as-is. It fits in one line.
    Ok,
    /// Wrapping does not provide any benefit.
    NoWrap,
    /// Wrapping allows the code to fit in one line.
    WrapForSingleLine,
    /// Wrapping allows more code to fit in the first line, but it is still multiple lines.
    WrapForLongerFirstLine,
    /// Wrapping exceeds the max width by a lesser amount.
    WrapForLessExcessWidth,
}

impl AstFormatter {
    /// Format with this function when choosing between continuing on the same line or wrapping to
    /// the next line.
    ///
    /// This will _not_ add a newline, but will increase the max width by the amount of extra
    /// horizontal space that _would_ be gained by wrapping. It also will turn off any width limit
    /// that is enabled for the current line. Finally, it uses single line mode to limit the
    /// experiment to the first line.
    ///
    /// See [`SimulateWrapResult`] for possible outcomes.
    ///
    /// N.B. This function relies on a subtle invariant that the given formatting scope will emit
    /// the entire first line except for trailing comments before returning a newline-related error
    /// when applicable.
    pub fn simulate_wrap(
        &self,
        offset: HSize,
        scope: impl FnOnce() -> FormatResult,
    ) -> FormatResult<SimulateWrapResult> {
        let max_width = self.constraints().max_width.get();
        let col = self.out.col();
        let wrap_indent_col = self.out.total_indent.get() + offset;
        let extra_width = col.checked_sub(wrap_indent_col).filter(|&w| w > 0);

        let result;
        let exceeded_width_limit;
        let used_extra_width;
        {
            let _guard = self.constraints().single_line.replace_guard(true);
            let _guard = extra_width.map(|extra_width| {
                let new_max_width = max_width.saturating_add(extra_width);
                self.constraints().max_width.replace_guard(new_max_width)
            });
            let width_limit_guard = self.width_limit_end_col().map(|end_col| {
                self.constraints().width_limit.replace_guard(Some(Rc::new(
                    WidthLimit {
                        end_col,
                        line: self.out.line(),
                        simulate: Some(WidthLimitSimulate::default()),
                    },
                )))
            });
            let _guard = self.recover_width_guard();

            result = scope();

            exceeded_width_limit = width_limit_guard.is_some()
                && self
                    .constraints()
                    .width_limit()
                    .map_or(false, |width_limit| {
                        width_limit.simulate.as_ref().is_some_and(|s| {
                            s.exceeded.get()
                        })
                    });
            used_extra_width = self.out.col() > max_width;
        };

        let simulate_result = match result {
            Ok(()) => {
                if used_extra_width || exceeded_width_limit {
                    SimulateWrapResult::WrapForSingleLine
                } else {
                    SimulateWrapResult::Ok
                }
            }
            Err(err) => match err.kind {
                FormatErrorKind::WidthLimitExceeded => SimulateWrapResult::WrapForLessExcessWidth,
                FormatErrorKind::Vertical(_) => {
                    if used_extra_width || exceeded_width_limit {
                        SimulateWrapResult::WrapForLongerFirstLine
                    } else {
                        SimulateWrapResult::NoWrap
                    }
                }
                _ => return Err(err),
            },
        };
        Ok(simulate_result)
    }
}
