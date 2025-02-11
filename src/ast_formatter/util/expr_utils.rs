use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;
use crate::rustc_span::Pos;
use crate::util::whitespace_utils::is_whitespace;
use rustc_ast::ast;

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
        let before_expr = &source[
            block.span.lo().to_usize() + 1..expr.span.lo().to_usize()
        ];
        let after_expr = &source[
            expr.span.hi().to_usize()..block.span.hi().to_usize() - 1
        ];
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
