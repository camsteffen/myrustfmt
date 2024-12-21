use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::error::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter {
    pub fn block(&self, block: &ast::Block) -> FormatResult {
        self.out.token_at("{", block.span.lo())?;
        self.block_after_open_brace(block)?;
        Ok(())
    }

    pub fn block_after_open_brace(&self, block: &ast::Block) -> FormatResult {
        self.block_generic_after_open_brace(&block.stmts, |stmt| self.stmt(stmt))
    }

    pub fn block_generic<T>(
        &self,
        items: &[T],
        format_item: impl Fn(&T) -> FormatResult,
    ) -> FormatResult {
        self.out.token_expect("{")?;
        self.block_generic_after_open_brace(items, format_item)?;
        Ok(())
    }

    pub fn block_generic_after_open_brace<T>(
        &self,
        items: &[T],
        format_item: impl Fn(&T) -> FormatResult,
    ) -> FormatResult {
        if !items.is_empty() {
            self.indented(|| {
                for item in items {
                    self.out.newline_indent()?;
                    format_item(item)?;
                }
                Ok(())
            })?;
            self.out.newline_indent()?;
        }
        self.out.token_expect("}")?;
        Ok(())
    }

    fn stmt(&self, stmt: &ast::Stmt) -> FormatResult {
        let hi = stmt.span.hi();
        let semicolon = move || self.out.token_end_at(";", hi);
        let semicolon = Tail::new(&semicolon);
        match &stmt.kind {
            ast::StmtKind::Let(local) => self.local(local, semicolon),
            ast::StmtKind::Item(item) => self.item(item),
            ast::StmtKind::Expr(expr) => self.expr(expr),
            ast::StmtKind::Semi(expr) => self.expr_tail(expr, semicolon),
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
