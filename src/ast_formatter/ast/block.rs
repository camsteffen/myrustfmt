use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::ast::item::MaybeItem;
use crate::ast_formatter::tail::Tail;
use crate::ast_utils::{jump_expr_kind, plain_block};
use crate::error::FormatResult;
use crate::util::whitespace_utils::is_whitespace;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;
use rustc_span::Pos;

impl AstFormatter {
    pub fn block_expr(&self, omit_open_brace: bool, block: &ast::Block) -> FormatResult {
        self.block_with_item_sorting(omit_open_brace, &block.stmts, |stmt| self.stmt(stmt))
    }

    pub fn block_expr_allow_horizontal(
        &self,
        label: Option<ast::Label>,
        block: &ast::Block,
        tail: Tail,
    ) -> FormatResult {
        self.label_colon(label)?;
        match block.rules {
            ast::BlockCheckMode::Default => {}
            ast::BlockCheckMode::Unsafe(_) => self.out.token_space("unsafe")?,
        }
        self.out.token("{")?;
        match self.try_into_expr_only_block(block) {
            None => {
                self.block_expr(true, block)?;
                self.tail(tail)?;
            }
            Some(expr_only_block) => {
                self.backtrack()
                    .next(|| {
                        self.with_single_line(|| {
                            self.expr_only_block_after_open_brace(expr_only_block)?;
                            self.tail(tail)?;
                            Ok(())
                        })
                    })
                    .next(|| {
                        self.block_expr(true, block)?;
                        self.tail(tail)?;
                        Ok(())
                    })
                    .result()?
            }
        }
        Ok(())
    }

    pub fn block<T>(
        &self,
        omit_open_brace: bool,
        list: &[T],
        format: impl Fn(&T) -> FormatResult,
    ) -> FormatResult {
        self.do_block(
            omit_open_brace,
            list.split_first().map(|(first, rest)| {
                move || {
                    format(first)?;
                    for item in rest {
                        self.out.newline_indent(VerticalWhitespaceMode::Between)?;
                        format(item)?;
                    }
                    Ok(())
                }
            }),
        )
    }

    pub fn block_with_item_sorting<T: MaybeItem>(
        &self,
        omit_open_brace: bool,
        list: &[T],
        format: impl Fn(&T) -> FormatResult,
    ) -> FormatResult {
        self.do_block(
            omit_open_brace,
            (!list.is_empty())
                .then_some(|| self.list_with_item_sorting(list, format)),
        )
    }

    fn do_block(
        &self,
        omit_open_brace: bool,
        contents: Option<impl FnOnce() -> FormatResult>,
    ) -> FormatResult {
        if !omit_open_brace {
            self.out.token("{")?;
        }
        match contents {
            None => self.enclosed_empty_after_opening("}")?,
            Some(contents) => self.enclosed_after_opening("}", contents)?,
        }
        Ok(())
    }

    pub fn stmt(&self, stmt: &ast::Stmt) -> FormatResult {
        match &stmt.kind {
            ast::StmtKind::Let(local) => self.local(local),
            ast::StmtKind::Item(item) => self.item(item),
            ast::StmtKind::Expr(expr) => {
                let tail = match expr.kind {
                    jump_expr_kind!() => self.tail_fn(|af| af.out.token_insert(";")),
                    _ => None,
                };
                self.expr_tail(expr, tail.as_ref())
            }
            ast::StmtKind::Semi(expr) => self.expr_tail(expr, self.tail_token(";").as_ref()),
            ast::StmtKind::Empty => self.out.token(";"),
            ast::StmtKind::MacCall(mac_call_stmt) => {
                self.with_attrs(&mac_call_stmt.attrs, stmt.span, || {
                    self.mac_call(&mac_call_stmt.mac)?;
                    match mac_call_stmt.style {
                        ast::MacStmtStyle::Semicolon => self.out.token(";")?,
                        ast::MacStmtStyle::Braces | ast::MacStmtStyle::NoBraces => {}
                    }
                    Ok(())
                })
            }
        }
    }

    // todo test removing and adding blocks when there are comments
    pub fn add_block(&self, contents: impl FnOnce() -> FormatResult) -> FormatResult {
        self.out.token_insert("{")?;
        self.enclosed_contents(contents)?;
        self.out.token_insert("}")?;
        Ok(())
    }

    // todo test removing and adding blocks when there are comments
    /// Wraps an expression in a multi-line block
    pub fn expr_add_block(&self, expr: &ast::Expr) -> FormatResult {
        self.add_block(|| self.expr(expr))
    }

    /// `{{{ expr }}}` -> `expr`
    pub fn skip_single_expr_blocks(
        &self,
        expr: &ast::Expr,
        format: impl FnOnce(&ast::Expr) -> FormatResult,
    ) -> FormatResult {
        self.skip_single_expr_blocks_tail(expr, None, |e, tail| {
            format(e)?;
            self.tail(tail)?;
            Ok(())
        })
    }

    /// `{{{ expr }}}` -> `expr`
    pub fn skip_single_expr_blocks_tail(
        &self,
        expr: &ast::Expr,
        tail: Tail,
        format: impl FnOnce(&ast::Expr, Tail) -> FormatResult,
    ) -> FormatResult {
        match plain_block(expr)
            .and_then(|b| self.try_into_expr_only_block(b))
        {
            None => format(expr, tail),
            Some(ExprOnlyBlock(inner)) => {
                self.out.skip_token("{")?;
                self.skip_single_expr_blocks_tail(
                    inner,
                    self.tail_fn(|af| {
                        af.out.skip_token("}")?;
                        self.tail(tail)?;
                        Ok(())
                    })
                    .as_ref(),
                    format,
                )?;
                Ok(())
            }
        }
    }

    pub fn is_block_empty(&self, block: &ast::Block) -> bool {
        if !block.stmts.is_empty() {
            return false;
        }
        let source = self.out.source_reader.source();
        let inside = &source[block.span.lo().to_usize() + 1..block.span.hi().to_usize() - 1];
        is_whitespace(inside)
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
        let source = self.out.source_reader.source();
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
