use rustc_ast::ast;

use crate::formatter::{FormatResult, Formatter};

impl<'a> Formatter<'a> {
    pub fn block(&mut self, block: &ast::Block) -> FormatResult {
        self.token("{", block.span.lo())?;
        if !block.stmts.is_empty() {
            self.out.increment_indent();
            let stmt_indent = self.out.current_indent();
            for stmt in &block.stmts {
                self.newline_indent()?;
                self.stmt(stmt)?;
                self.out.set_indent(stmt_indent);
            }
            self.out.decrement_indent();
            self.newline_indent()?;
        }
        self.debug_span(block.span);
        self.debug_pos();
        self.token_with_end("}", block.span.hi())?;
        Ok(())
    }

    fn stmt(&mut self, stmt: &ast::Stmt) -> FormatResult {
        match &stmt.kind {
            ast::StmtKind::Let(local) => self.with_reserved_width(";".len(), |this| {
                this.local(local)?;
                this.token_expect(";")?;
                Ok(())
            }),
            ast::StmtKind::Item(_) => Ok(()),
            ast::StmtKind::Expr(_) => Ok(()),
            ast::StmtKind::Semi(_) => Ok(()),
            ast::StmtKind::Empty => Ok(()),
            ast::StmtKind::MacCall(_) => Ok(()),
        }
    }
}
