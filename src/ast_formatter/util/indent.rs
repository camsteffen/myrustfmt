use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::constraints::VerticalShape;
use crate::error::FormatResult;
use crate::util::cell_ext::CellExt;

impl AstFormatter {
    pub fn indented<T>(&self, format: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        let indent = self.out.indent.get() + INDENT_WIDTH;
        self.out.indent.with_replaced(indent, || {
            match self.constraints().vertical.get() {
                // SingleLine must be preserved
                VerticalShape::SingleLine | VerticalShape::Unrestricted => format(),
                // For any other shape, indentation "resets" the shape to Unrestricted
                // since the shape is only concerned with where code touches the left margin.
                _ => self.constraints().vertical.with_replaced(VerticalShape::Unrestricted, format),
            }
        })
    }

    pub fn indented_optional(
        &self,
        should_indent: bool,
        format: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        if !should_indent {
            return format();
        }
        self.indented(format)
    }
}
