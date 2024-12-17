use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::source_formatter::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter<'a> {
    pub fn block(&mut self, block: &ast::Block, end: Tail) -> FormatResult {
        self.out.token_at("{", block.span.lo())?;
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
        self.tail(&end)
    }

    fn stmt(&mut self, stmt: &ast::Stmt) -> FormatResult {
        let hi = stmt.span.hi();
        let semicolon = Tail::new(move |this| this.out.token_end_at(";", hi));
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
