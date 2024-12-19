use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::source_formatter::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter {
    pub fn block(&self, block: &ast::Block, end: Tail<'_>) -> FormatResult {
        self.out.token_at("{", block.span.lo())?;
        self.block_after_open_brace(block, end)?;
        Ok(())
    }

    pub fn block_after_open_brace(&self, block: &ast::Block, end: Tail<'_>) -> FormatResult {
        if !block.stmts.is_empty() {
            self.indented(|this| {
                for stmt in &block.stmts {
                    this.out.newline_indent()?;
                    this.stmt(stmt)?;
                }
                Ok(())
            })?;
            self.out.newline_indent()?;
        }
        self.out.token_end_at("}", block.span.hi())?;
        self.tail(end)?;
        Ok(())
    }

    fn stmt(&self, stmt: &ast::Stmt) -> FormatResult {
        let hi = stmt.span.hi();
        let semicolon = move |this: &AstFormatter| this.out.token_end_at(";", hi);
        let semicolon = Tail::new(&semicolon);
        match &stmt.kind {
            ast::StmtKind::Let(local) => self.local(local, semicolon),
            ast::StmtKind::Item(_) => todo!(),
            ast::StmtKind::Expr(expr) => self.expr(expr, Tail::NONE),
            ast::StmtKind::Semi(expr) => self.expr(expr, semicolon),
            ast::StmtKind::Empty => self.out.token_expect(";"),
            ast::StmtKind::MacCall(mac_call_stmt) => {
                self.attrs(&mac_call_stmt.attrs)?;
                match mac_call_stmt.style {
                    ast::MacStmtStyle::Semicolon => self.mac_call(&mac_call_stmt.mac, semicolon),
                    ast::MacStmtStyle::Braces | ast::MacStmtStyle::NoBraces => {
                        self.mac_call(&mac_call_stmt.mac, Tail::NONE)
                    }
                }
            }
        }
    }
}
