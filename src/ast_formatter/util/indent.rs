use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::constraints::MultiLineShape;
use crate::error::FormatResult;
use crate::util::cell_ext::CellExt;

impl AstFormatter {
    pub fn indented<T>(&self, f: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        let indent = self.out.indent.get() + INDENT_WIDTH;
        self.out.indent.with_replaced(indent, || {
            let shape = self.constraints().borrow().multi_line;
            match shape {
                MultiLineShape::SingleLine | MultiLineShape::Unrestricted => f(),
                _ => {
                    self.constraints()
                        .with_multi_line_shape_replaced(MultiLineShape::Unrestricted, f)
                }
            }
        })
    }

    pub fn indented_optional(
        &self,
        should_indent: bool,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        if !should_indent {
            return f();
        }
        self.indented(f)
    }
}
