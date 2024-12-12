use crate::ast_formatter::AstFormatter;
use crate::source_formatter::FormatResult;
use rustc_ast::ast;
use rustc_span::symbol::Ident;

impl<'a> AstFormatter<'a> {
    pub fn ident(&mut self, ident: Ident) -> FormatResult {
        self.out.token_from_source(ident.span)
    }
    
    pub fn mutability(&mut self, mutability: ast::Mutability) -> FormatResult {
        match mutability {
            ast::Mutability::Mut => {
                self.out.token_expect("mut")?;
                self.out.space()?;
                Ok(())
            },
            ast::Mutability::Not => Ok(())
        }
    }

    pub fn strlit(&mut self, strlit: &ast::StrLit) -> FormatResult {
        self.out.token_from_source(strlit.span)
    }

    pub fn safety(&mut self, safety: &ast::Safety) -> FormatResult {
        match *safety {
            ast::Safety::Unsafe(span) => {
                let pos = span.lo();
                self.out.token_at_space("unsafe", pos)
            }
            ast::Safety::Safe(span) => {
                let pos = span.lo();
                self.out.token_at_space("safe", pos)
            }
            ast::Safety::Default => Ok(()),
        }
    }
}
