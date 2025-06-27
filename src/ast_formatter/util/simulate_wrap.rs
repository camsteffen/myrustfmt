use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::error::{FormatErrorKind, FormatResult};
use crate::num::HSize;
use crate::util::cell_ext::CellExt;

#[derive(Clone, Copy, Debug)]
pub enum SimulateWrapResult {
    /// The result may be used as-is. It fits in one line.
    Ok,
    /// Wrapping does not provide any benefit
    NoWrap,
    /// Wrapping allows the code to fit in one line.
    WrapForSingleLine,
    /// Wrapping allows more code to fit in the first line, but it is multiple lines.
    WrapForLongerFirstLine,
    /// Wrapping exceeds the max width, but by a lesser amount.
    WrapForLessExcessWidth,
}

impl AstFormatter {
    /// Format with this function when choosing between continuing on the same line or wrapping to
    /// the next line and adding an indentation (think "ENTER, TAB").
    ///
    /// This will _not_ add a newline or indent, but will increase the max width by the amount of
    /// extra horizontal space that _would_ be gained by wrapping. It also will turn off any width
    /// limit that is enabled for the current line. Finally, it uses single line mode to limit the
    /// experiment to the first line.
    ///
    /// N.B. This function relies on a subtle invariant that the given formatting scope will emit
    /// the entire first line except for trailing comments before returning a newline-related error
    /// when applicable.
    ///
    /// When the extra width is used, this means one of two things: either the extra width allowed
    /// for a different formatting strategy with more code on the first line, or the extra width was
    /// strictly required to fit the code at all. This function is useful when these two cases are
    /// handled in the same way.
    pub fn simulate_wrap_indent(
        &self,
        offset: HSize,
        scope: impl FnOnce() -> FormatResult,
    ) -> FormatResult<SimulateWrapResult> {
        let with_single_line_and_no_width_limit = || {
            let _guard = self.constraints().single_line.replace_guard(true);
            let _guard = self.constraints().width_limit.replace_guard(None);
            scope()
        };
        let (result, used_extra_width) = self.out.with_recover_width(|| {
            let col = self.out.col();
            let wrap_indent_col = self.out.total_indent.get() + INDENT_WIDTH + offset;
            match col.checked_sub(wrap_indent_col) {
                None | Some(0) => {
                    let result = with_single_line_and_no_width_limit();
                    (result, false)
                }
                Some(extra_width) => {
                    let max_width = self.constraints().max_width.get();
                    let max_width_extra = max_width.saturating_add(extra_width);
                    let result = self
                        .constraints()
                        .max_width
                        .with_replaced(max_width_extra, with_single_line_and_no_width_limit);
                    let used_extra_width = self.out.col() > max_width;
                    (result, used_extra_width)
                }
            }
        });
        let exceeded_width_limit = self.constraints().width_limit.get().is_some_and(|wl| {
            wl.line == self.out.line() && self.out.col() > wl.end_col.get()
        });
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
