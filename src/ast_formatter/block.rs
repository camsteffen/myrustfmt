use crate::ast_formatter::AstFormatter;
use crate::source_formatter::{FormatResult};
use rustc_ast::ast;

impl<'a> AstFormatter<'a> {
    pub fn block(&mut self, block: &ast::Block) -> FormatResult {
        self.out.token_at("{", block.span.lo())?;
        if !block.stmts.is_empty() {
            self.with_indent(|this| {
                for stmt in &block.stmts {
                    this.out.newline_indent()?;
                    this.stmt(stmt)?;
                }
                Ok(())
            })?;
            self.out.newline_indent()?;
        }
        self.out.token_end_at("}", block.span.hi())?;
        Ok(())
    }

    fn stmt(&mut self, stmt: &ast::Stmt) -> FormatResult {
        match &stmt.kind {
            ast::StmtKind::Let(local) => {
                self.with_reserved_width(";".len(), |this| this.local(local))?;
                self.out.token_expect(";")?;
                Ok(())
            }
            ast::StmtKind::Item(_) => todo!(),
            ast::StmtKind::Expr(_) => todo!(),
            ast::StmtKind::Semi(_) => todo!(),
            ast::StmtKind::Empty => todo!(),
            ast::StmtKind::MacCall(_) => todo!(),
        }
    }
}
