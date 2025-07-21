use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::source_formatter::SourceFormatter;

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
    pub fn indent_guard(&self) -> IndentGuard<'_> {
        let out = &self.out;
        out.total_indent.set(out.total_indent.get() + INDENT_WIDTH);
        IndentGuard { out }
    }
}
