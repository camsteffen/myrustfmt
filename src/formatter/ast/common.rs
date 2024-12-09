use rustc_ast::ast;
use rustc_span::symbol::Ident;

use crate::formatter::{FormatResult, Formatter};

impl<'a> Formatter<'a> {
    pub fn ident(&mut self, ident: Ident) -> FormatResult {
        self.token_from_source(ident.span)
    }

    pub fn strlit(&mut self, strlit: &ast::StrLit) {
        self.token_from_source(strlit.span);
    }
}
