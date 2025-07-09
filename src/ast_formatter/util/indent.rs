use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::error::FormatResult;
use crate::source_formatter::SourceFormatter;
use crate::util::cell_ext::CellExt;

pub struct IndentGuard<'a> {
    out: &'a SourceFormatter,
}

impl Drop for IndentGuard<'_> {
    fn drop(&mut self) {
        self.out
            .total_indent
            .set(self.out.total_indent.get() - INDENT_WIDTH);
    }
}

impl AstFormatter {
    pub fn begin_indent(&self) -> IndentGuard<'_> {
        let out = &self.out;
        out.total_indent.set(out.total_indent.get() + INDENT_WIDTH);
        IndentGuard { out }
    }

    pub fn indented<T>(&self, scope: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        let _guard = self.begin_indent();
        scope()
    }

    pub fn deindented<T>(&self, scope: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        let _guard = self.out.total_indent.replace_guard(
            self.out.total_indent.get() - INDENT_WIDTH,
        );
        scope()
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
