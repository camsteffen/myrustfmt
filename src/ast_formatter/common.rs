use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;
use rustc_ast::ast;
use rustc_span::symbol::Ident;

impl AstFormatter {
    pub fn ident(&self, ident: Ident) -> FormatResult {
        self.out.token_from_source(ident.span)
    }

    pub fn mutability(&self, mutability: ast::Mutability) -> FormatResult {
        match mutability {
            ast::Mutability::Mut => self.out.token_space("mut"),
            ast::Mutability::Not => Ok(()),
        }
    }

    pub fn strlit(&self, strlit: &ast::StrLit) -> FormatResult {
        self.out.token_from_source(strlit.span)
    }

    pub fn safety(&self, safety: &ast::Safety) -> FormatResult {
        match *safety {
            ast::Safety::Unsafe(_) => self.out.token_space("unsafe"),
            ast::Safety::Safe(_) => self.out.token_space("safe"),
            ast::Safety::Default => Ok(()),
        }
    }
}
