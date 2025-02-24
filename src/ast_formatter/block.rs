use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::tail::Tail;
use crate::ast_utils::{control_flow_expr_kind, plain_block};
use crate::error::FormatResult;
use crate::util::whitespace_utils::is_whitespace;
use rustc_ast::ast;
use rustc_span::Pos;

impl AstFormatter {
    pub fn block_expr(
        &self,
        label: Option<ast::Label>,
        block: &ast::Block,
        tail: &Tail,
    ) -> FormatResult {
        self.label(label)?;
        self.out.token("{")?;
        match self.try_into_expr_only_block(block) {
            None => {
                self.block_generic_after_open_brace(&block.stmts, |stmt| self.stmt(stmt))?;
                self.tail(tail)?;
            }
            Some(expr_only_block) => {
                self.backtrack()
                    .next(|| {
                        self.with_single_line(
                            || self.expr_only_block_after_open_brace(expr_only_block),
                        )?;
                        self.tail(tail)?;
                        Ok(())
                    })
                    .otherwise(|| {
                        self.block_generic_after_open_brace(&block.stmts, |stmt| self.stmt(stmt))?;
                        self.tail(tail)?;
                        Ok(())
                    })?
            }
        }
        Ok(())
    }

    pub fn block_separate_lines(&self, block: &ast::Block) -> FormatResult {
        self.out.token("{")?;
        self.block_separate_lines_after_open_brace(block)?;
        Ok(())
    }

    pub fn block_separate_lines_after_open_brace(&self, block: &ast::Block) -> FormatResult {
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

    // todo test removing and adding blocks when there are comments
    /// Wraps an expression in a multi-line block
    pub fn expr_add_block(&self, expr: &ast::Expr) -> FormatResult {
        self.out.token_insert("{")?;
        self.embraced_inside(|| self.expr(expr))?;
        self.out.token_insert("}")?;
        Ok(())
    }

    /// `{{{ expr }}}` -> `expr`
    pub fn skip_single_expr_blocks(
        &self,
        expr: &ast::Expr,
        format: impl FnOnce(&ast::Expr) -> FormatResult,
    ) -> FormatResult {
        self.skip_single_expr_blocks_tail(expr, Tail::none(), |e, tail| {
            format(e)?;
            self.tail(tail)?;
            Ok(())
        })
    }

    /// `{{{ expr }}}` -> `expr`
    pub fn skip_single_expr_blocks_tail(
        &self,
        expr: &ast::Expr,
        tail: &Tail,
        format: impl FnOnce(&ast::Expr, &Tail) -> FormatResult,
    ) -> FormatResult {
        match plain_block(expr)
            .and_then(|b| self.try_into_expr_only_block(b))
        {
            None => format(expr, tail),
            Some(ExprOnlyBlock(inner)) => {
                self.out.skip_token("{")?;
                self.skip_single_expr_blocks_tail(
                    inner,
                    &Tail::func(|af| {
                        af.out.skip_token("}")?;
                        self.tail(tail)?;
                        Ok(())
                    }),
                    format,
                )?;
                Ok(())
            }
        }
    }
}

/// A block that contains only a single expression, no semicolon, and no comments.
/// This may be written on one line.
#[derive(Clone, Copy)]
pub struct ExprOnlyBlock<'a>(pub &'a ast::Expr);

impl AstFormatter {
    /// `{ expr }` -> `expr`
    ///
    /// If a block contains only an expression, return the expression.
    /// This may be used together with `plain_block`.
    pub fn try_into_expr_only_block<'a>(&self, block: &'a ast::Block) -> Option<ExprOnlyBlock<'a>> {
        let [stmt] = &block.stmts[..] else {
            return None;
        };
        let ast::StmtKind::Expr(expr) = &stmt.kind else {
            return None;
        };
        if !expr.attrs.is_empty() {
            return None;
        }
        let source = self.out.source();
        let before_expr = &source[block.span.lo().to_usize() + 1..expr.span.lo().to_usize()];
        let after_expr = &source[expr.span.hi().to_usize()..block.span.hi().to_usize() - 1];
        if !(is_whitespace(before_expr) && is_whitespace(after_expr)) {
            // there are comments before or after the expression
            return None;
        }
        Some(ExprOnlyBlock(expr))
    }

    pub fn expr_only_block(&self, expr_only_block: ExprOnlyBlock) -> FormatResult {
        self.out.token("{")?;
        self.expr_only_block_after_open_brace(expr_only_block)?;
        Ok(())
    }

    pub fn expr_only_block_after_open_brace(&self, expr_only_block: ExprOnlyBlock) -> FormatResult {
        self.out.space()?;
        self.expr(expr_only_block.0)?;
        self.out.space_token("}")?;
        Ok(())
    }
}
