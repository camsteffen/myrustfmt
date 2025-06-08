use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::error::{FormatErrorKind, FormatResult};
use crate::util::cell_ext::CellExt;

#[derive(Debug)]
pub enum SimulateWrapResult {
    Ok,
    NoWrap,
    Wrap { single_line: bool },
}

impl AstFormatter {
    /// Format with this function when choosing between continuing on the same line or wrapping to
    /// the next line and adding an indentation. It will format on the same line, but will increase
    /// the max width by the amount of extra horizontal space that _would_ be gained by wrapping.
    /// See [`SimulateWrapResult`] for all the possible outcomes.
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
        wrap_for_longer_first_line: bool,
        scope: impl FnOnce() -> FormatResult,
    ) -> SimulateWrapResult {
        let with_single_line_and_no_width_limit =
            || self.with_single_line(|| self.constraints().width_limit.with_replaced(None, scope));
        self.out.with_recover_width(|| {
            let col = self.out.col();
            let wrap_indent_col = self.out.total_indent.get() + INDENT_WIDTH;
            let (result, used_extra_width) = match col.checked_sub(wrap_indent_col) {
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
            };
            let exceeded_width_limit = self
                .constraints()
                .width_limit
                .get()
                .is_some_and(|wl| wl.line == self.out.line() && self.out.col() > wl.end_col.get());
            match (result, used_extra_width || exceeded_width_limit) {
                // simple case - we can use the result as is
                (Ok(()), false) => SimulateWrapResult::Ok,
                // the output will fit in a single line if wrapped
                // todo don't wrap for single line if it's a closure or block
                (Ok(()), true) => SimulateWrapResult::Wrap { single_line: true },
                // If we used extra width and still exceeded the max width or a width limit,
                // wrapping is preferred in order to exceed the max width by a lesser amount.
                // If we used extra width and encountered a newline-related error, we can infer that
                // wrapping allows for more code to fit in the first line.
                (Err(e), true)
                    if e.kind == FormatErrorKind::WidthLimitExceeded
                        || wrap_for_longer_first_line =>
                    {
                        SimulateWrapResult::Wrap { single_line: false }
                    }
                // In all other cases, we don't necessarily want to wrap
                (Err(_), _) => SimulateWrapResult::NoWrap,
            }
        })
    }
}
