use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::constraints::Shape;
use crate::error::{ConstraintErrorKind, FormatResult};
use crate::source_formatter::Lookahead;

#[derive(Debug)]
pub enum SimulateWrapResult {
    /// Output fits in one line and extra width not used -- okay to use the output.
    Ok,
    /// Output would fit in one line if wrapped.
    WrapToFitInOneLine,
    /// Multiple lines required
    MultiLine { longer_first_line_with_wrap: bool },
    /// The output exceeds max width with or without wrapping
    TooWide { less_so_with_wrap: bool },
}

#[derive(Debug)]
pub enum SimulateWrapDecision {
    Keep,
    SameLine,
    Wrap { single_line: Option<Lookahead> },
}

impl AstFormatter {
    /// Format with this function when choosing between continuing on the same line or wrapping to
    /// the next line and adding an indentation. It will format on the same line, but will increase
    /// the max width by the amount of extra horizontal space that _would_ be gained by wrapping.
    /// See [`SimulateWrapResult`] for all the possible outcomes.
    /// 
    /// N.B. This function relies on a subtle invariant that the given formatting scope will emit
    /// the entire first line except for trailing comments before returning a newline-related error
    /// when applicable. See also [`Shape`].
    ///
    /// When the extra width is used, this means one of two things: either the extra width allowed
    /// for a different formatting strategy with more code on the first line, or the extra width was
    /// strictly required to fit the code at all. This function is useful when these two cases are
    /// handled in the same way.
    pub fn simulate_wrap_indent_first_line(
        &self,
        bias_to_wrap: bool,
        scope: impl FnOnce() -> FormatResult,
    ) -> SimulateWrapDecision {
        let checkpoint = self.out.checkpoint();
        self.out.with_recoverable_width(|| {
            let col = self.out.col();
            // the starting position if we wrapped to the next line and indented
            let wrap_indent_col = self.out.total_indent.get() + INDENT_WIDTH;
            let extra_width = col.checked_sub(wrap_indent_col).filter(|&n| n > 0);
            let (result, used_extra_width) = match extra_width {
                None => {
                    // wrap-indent doesn't gain additional width
                    let result = self.with_replace_shape(Shape::SingleLine, scope);
                    (result, false)
                }
                Some(extra_width) => {
                    let max_width = self.out.current_max_width();
                    let max_width_extra = max_width.saturating_add(extra_width);
                    let result = self.with_replace_shape(Shape::SingleLine, || {
                        self.with_replace_max_width(max_width_extra, scope)
                    });
                    let used_extra_width = self.out.col() > max_width;
                    (result, used_extra_width)
                }
            };
            let result = match (result, used_extra_width) {
                (Ok(()), false) => SimulateWrapResult::Ok,
                (Ok(()), true) => SimulateWrapResult::WrapToFitInOneLine,
                (Err(e), _) if e.kind == ConstraintErrorKind::WidthLimitExceeded => {
                    SimulateWrapResult::TooWide {
                        less_so_with_wrap: extra_width.is_some(),
                    }
                }
                (Err(_), _) => SimulateWrapResult::MultiLine {
                    longer_first_line_with_wrap: used_extra_width,
                },
            };
            // todo test all possibilities
            // todo collapse the two enums?
            match result {
                SimulateWrapResult::Ok => SimulateWrapDecision::Keep,
                SimulateWrapResult::WrapToFitInOneLine => SimulateWrapDecision::Wrap {
                    single_line: Some(self.out.capture_lookahead(&checkpoint)),
                },
                SimulateWrapResult::MultiLine {
                    longer_first_line_with_wrap,
                } => {
                    if bias_to_wrap && longer_first_line_with_wrap {
                        SimulateWrapDecision::Wrap { single_line: None }
                    } else {
                        SimulateWrapDecision::SameLine
                    }
                }
                SimulateWrapResult::TooWide { less_so_with_wrap } => {
                    if bias_to_wrap && less_so_with_wrap {
                        SimulateWrapDecision::Wrap { single_line: None }
                    } else {
                        SimulateWrapDecision::SameLine
                    }
                }
            }
        })
    }
}
