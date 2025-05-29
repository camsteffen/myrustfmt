use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::error::FormatResult;
use crate::source_formatter::SourceFormatter;
use crate::util::cell_ext::CellExt;
use enumset::EnumSet;

pub struct IndentGuard<'a> {
    out: &'a SourceFormatter,
}

impl Drop for IndentGuard<'_> {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            self.out
                .total_indent
                .set(self.out.total_indent.get() - INDENT_WIDTH);
        }
    }
}

impl AstFormatter {
    pub fn begin_indent(&self) -> IndentGuard {
        let out = &self.out;
        out.total_indent.set(out.total_indent.get() + INDENT_WIDTH);
        IndentGuard { out }
    }

    pub fn indented<T>(&self, format: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        let indent = self.out.total_indent.get() + INDENT_WIDTH;
        self.out.total_indent.with_replaced(indent, || {
            self.constraints().disallowed_vstructs.with_replaced(EnumSet::new(), format)
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
