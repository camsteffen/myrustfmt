use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;
use rustc_ast::ast;
use crate::ast_utils::control_flow_expr_kind;

impl AstFormatter {
    pub fn block(&self, block: &ast::Block) -> FormatResult {
        self.out.token("{")?;
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
        self.out.token("{")?;
        self.block_generic_after_open_brace(items, format_item)?;
        Ok(())
    }

    pub fn block_generic_after_open_brace<T>(
        &self,
        items: &[T],
        format_item: impl Fn(&T) -> FormatResult,
    ) -> FormatResult {
        match items {
            [] => self.embraced_empty_after_opening("}"),
            [first, rest @ ..] => self.embraced_after_opening("}", || {
                format_item(first)?;
                for item in rest {
                    self.out.newline_between_indent()?;
                    format_item(item)?;
                }
                Ok(())
            }),
        }
    }

    pub fn stmt(&self, stmt: &ast::Stmt) -> FormatResult {
        match &stmt.kind {
            ast::StmtKind::Let(local) => self.local(local),
            ast::StmtKind::Item(item) => self.item(item),
            ast::StmtKind::Expr(expr) => {
                let tail = if matches!(expr.kind, control_flow_expr_kind!()) {
                    &Tail::token_insert(";")
                } else {
                    Tail::none()
                };
                self.expr_tail(expr, tail)
            }
            ast::StmtKind::Semi(expr) => self.expr_tail(expr, &Tail::token(";")),
            ast::StmtKind::Empty => self.out.token(";"),
            ast::StmtKind::MacCall(mac_call_stmt) => {
                self.with_attrs(&mac_call_stmt.attrs, stmt.span, || {
                    match mac_call_stmt.style {
                        ast::MacStmtStyle::Semicolon => {
                            self.mac_call(&mac_call_stmt.mac)?;
                            self.out.token(";")?;
                            Ok(())
                        }
                        ast::MacStmtStyle::Braces | ast::MacStmtStyle::NoBraces => {
                            self.mac_call(&mac_call_stmt.mac)
                        }
                    }
                })
            }
        }
    }
}
