use crate::ast_formatter::AstFormatter;
use crate::source_formatter::{FormatResult, SourceFormatter};
use rustc_ast::ast;
use rustc_span::symbol::Ident;

impl<'a> AstFormatter<'a> {
    pub fn ident(&mut self, ident: Ident) -> FormatResult {
        self.out.token_from_source(ident.span)
    }

    pub fn strlit(&mut self, strlit: &ast::StrLit) -> FormatResult {
        self.out.token_from_source(strlit.span)
    }
}
