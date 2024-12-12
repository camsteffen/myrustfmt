use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::{EndReserved, EndWidth, drop_end_reserved};
use crate::source_formatter::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter<'a> {
    pub fn block(&mut self, block: &ast::Block) -> FormatResult {
        self.block_end(block, EndWidth::ZERO).map(drop_end_reserved)
    }

    pub fn block_end(&mut self, block: &ast::Block, end: EndWidth) -> FormatResult<EndReserved> {
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
        self.reserve_end(end)
    }

    fn stmt(&mut self, stmt: &ast::Stmt) -> FormatResult {
        match &stmt.kind {
            ast::StmtKind::Let(local) => {
                self.with_end_width(";".len(), |this, end| this.local(local, end))?;
                self.out.token_expect(";")?;
                Ok(())
            }
            ast::StmtKind::Item(_) => todo!(),
            ast::StmtKind::Expr(expr) => self.expr(expr),
            ast::StmtKind::Semi(expr) => {
                self.with_end_width(";".len(), |this, end| this.expr_end(expr, end))?;
                self.out.token_expect(";")?;
                Ok(())
            }
            ast::StmtKind::Empty => self.out.token_expect(";"),
            ast::StmtKind::MacCall(mac_call_stmt) => {
                self.attrs(&mac_call_stmt.attrs)?;
                match mac_call_stmt.style {
                    ast::MacStmtStyle::Semicolon => {
                        self.with_end_width(";".len(), |this, end| {
                            this.mac_call_end(&mac_call_stmt.mac, end)
                        })?;
                        self.out.token_expect(";")?
                    }
                    ast::MacStmtStyle::Braces | ast::MacStmtStyle::NoBraces => {
                        self.mac_call(&mac_call_stmt.mac)?;
                    }
                }
                Ok(())
            }
        }
    }
}
