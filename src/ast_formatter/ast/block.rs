use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::ast::item::MaybeItem;
use crate::ast_formatter::ast::r#macro::MacCallSemi;
use crate::ast_formatter::tail::Tail;
use crate::ast_utils::{is_jump_expr, plain_block};
use crate::error::FormatResult;
use crate::util::whitespace_utils::{is_whitespace, is_whitespace_or_semicolon};
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;
use rustc_span::Pos;

impl AstFormatter {
    pub fn block_expr(&self, omit_open_bracket: bool, block: &ast::Block) -> FormatResult {
        self.block_with_item_sorting(omit_open_bracket, &block.stmts, |stmt| self.stmt(stmt))
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
        match self.try_into_optional_block(block) {
            None => {
                self.block_expr(true, block)?;
                self.tail(tail)?;
            }
            Some(expr_only_block) => {
                self.backtrack()
                    .next(|_| {
                        self.with_single_line(|| {
                            self.optional_block_horizontal_after_open_brace(expr_only_block)?;
                            self.tail(tail)?;
                            Ok(())
                        })
                    })
                    .next(|_| {
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
            (!list.is_empty()).then_some(|| self.list_with_item_sorting(list, format)),
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
            None => self.enclosed_empty_contents()?,
            Some(contents) => self.enclosed_contents(contents)?,
        }
        self.out.token("}")?;
        Ok(())
    }

    pub fn stmt(&self, stmt: &ast::Stmt) -> FormatResult {
        match &stmt.kind {
            ast::StmtKind::Let(local) => self.local(local),
            ast::StmtKind::Item(item) => self.item(item),
            ast::StmtKind::Expr(expr) => self.expr_stmt(expr),
            ast::StmtKind::Semi(expr) => self.expr_tail(expr, Some(&self.tail_token(";"))),
            ast::StmtKind::Empty => self.out.token(";"),
            ast::StmtKind::MacCall(mac_call_stmt) => {
                self.with_attrs(&mac_call_stmt.attrs, stmt.span.into(), || {
                    self.macro_call(
                        &mac_call_stmt.mac,
                        match mac_call_stmt.style {
                            ast::MacStmtStyle::Semicolon => MacCallSemi::Yes,
                            ast::MacStmtStyle::Braces | ast::MacStmtStyle::NoBraces => {
                                MacCallSemi::No
                            }
                        },
                        None,
                    )
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
        match plain_block(expr).and_then(|b| self.try_into_optional_block(b)) {
            None => format(expr, tail),
            Some(opt_block) => {
                self.out.token_skip("{")?;
                let (inner, semi) = match opt_block {
                    OptionalBlock::Expr(inner) => (inner, false),
                    OptionalBlock::JumpExprSemi(inner) => (inner, true),
                };
                self.skip_single_expr_blocks_tail(
                    inner,
                    Some(&self.tail_fn(|af| {
                        if semi {
                            af.out.token_skip(";")?;
                        }
                        af.out.token_skip("}")?;
                        self.tail(tail)?;
                        Ok(())
                    })),
                    format,
                )?;
                Ok(())
            }
        }
    }

    pub fn is_block_empty(&self, block: &ast::Block) -> bool {
        // todo handle StmtKind::Empty
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
pub enum OptionalBlock<'a> {
    Expr(&'a ast::Expr),
    /// like `return x;`
    JumpExprSemi(&'a ast::Expr),
}

impl AstFormatter {
    /// `{ expr }` -> `expr`
    ///
    /// If a block contains only an expression, return the expression.
    /// This may be used together with `plain_block`.
    pub fn try_into_optional_block<'a>(&self, block: &'a ast::Block) -> Option<OptionalBlock<'a>> {
        let [stmt] = &block.stmts[..] else {
            return None;
        };
        // todo handle StmtKind::Empty
        let (expr, is_jump_semi) = match &stmt.kind {
            ast::StmtKind::Expr(expr) => (expr, false),
            ast::StmtKind::Semi(expr) if is_jump_expr(expr) => (expr, true),
            _ => return None,
        };
        if !expr.attrs.is_empty() {
            return None;
        }
        let source = self.out.source_reader.source();
        let before_expr = &source[block.span.lo().to_usize() + 1..expr.span.lo().to_usize()];
        let after_expr = &source[expr.span.hi().to_usize()..block.span.hi().to_usize() - 1];
        if !(is_whitespace(before_expr) && is_whitespace_or_semicolon(after_expr)) {
            // there are comments before or after the expression
            return None;
        }
        let opt_block = if is_jump_semi {
            OptionalBlock::JumpExprSemi(expr)
        } else {
            OptionalBlock::Expr(expr)
        };
        Some(opt_block)
    }

    pub fn optional_block_horizontal(&self, opt_block: OptionalBlock) -> FormatResult {
        self.out.token("{")?;
        self.optional_block_horizontal_after_open_brace(opt_block)?;
        Ok(())
    }

    pub fn optional_block_horizontal_after_open_brace(
        &self,
        opt_block: OptionalBlock,
    ) -> FormatResult {
        self.out.space()?;
        match opt_block {
            OptionalBlock::Expr(expr) => self.expr(expr)?,
            OptionalBlock::JumpExprSemi(expr) => {
                self.expr(expr)?;
                self.out.token_skip(";")?;
            }
        }
        self.out.space_token("}")?;
        Ok(())
    }
}
