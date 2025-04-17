use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::constraints::VerticalShape;

impl AstFormatter {
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
        let next_line_start = self.out.total_indent.get() + INDENT_WIDTH;
        let Some(extra_width) = start.checked_sub(next_line_start).filter(|&w| w > 0) else {
            let result = self.with_replace_vertical_shape(VerticalShape::SingleLine, scope);
            return (false, result);
        };
        let max_width_prev = self.out.current_max_width();
        let max_width = max_width_prev.saturating_add(extra_width);
        let result = self.with_replace_vertical_shape(VerticalShape::SingleLine, || self.with_replace_max_width(max_width, scope));
        let used_extra_width = self.out.last_line_len() > max_width_prev;
        (used_extra_width, result)
    }
}
