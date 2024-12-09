use crate::ast_formatter::AstFormatter;
use crate::source_formatter::{FormatResult, SourceFormatter};
use rustc_ast::ast;
use rustc_span::symbol::Ident;

impl<'a> AstFormatter<'a> {
    pub fn ident(&mut self, ident: Ident) -> FormatResult {
        let span = ident.span;
        self.out.token_from_source(span)
    }

    pub fn strlit(&mut self, strlit: &ast::StrLit) {
        let span = strlit.span;
        self.out.token_from_source(span);
    }
}
