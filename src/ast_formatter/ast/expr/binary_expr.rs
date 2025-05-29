use crate::ast_formatter::tail::Tail;
use crate::ast_formatter::{AstFormatter, INDENT_WIDTH};
use crate::error::FormatResult;

use crate::constraints::VStruct;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;
use rustc_span::source_map::Spanned;

impl AstFormatter {
    pub fn binary_expr(
        &self,
        left: &ast::Expr,
        right: &ast::Expr,
        op: Spanned<ast::BinOpKind>,
        tail: Tail,
    ) -> FormatResult {
        let (first, chain) = collect_binary_expr_chain(left, right, op);
        let first_line = self.out.line();
        self.expr(first)?;
        self.has_vstruct(VStruct::NonBlockIndent, || {
            let mut chain = chain.as_slice();
            let indent_margin = self.out.total_indent.get() + INDENT_WIDTH;
            let indent_guard = loop {
                let (op, expr) = if self.out.col() < indent_margin
                    && let Some(next) = chain.split_off_first()
                {
                    next
                } else {
                    break None;
                };
                let indent_guard = self.space_or_wrap_indent_then(|| {
                    self.out.token_space(op.as_str())?;
                    self.expr(expr)?;
                    Ok(())
                })?;
                if let Some(indent_guard) = indent_guard {
                    break Some(indent_guard);
                }
            };
            if chain.is_empty() {
                if let Some(indent_guard) = indent_guard {
                    indent_guard.close();
                }
                return self.tail(tail);
            }
            self.backtrack()
                // format all on one line, only if the first item fits in one line
                .next_if(self.out.line() == first_line, || {
                    self.with_single_line(|| {
                        for (op, expr) in chain {
                            self.out.space_token_space(op.as_str())?;
                            self.expr(expr)?;
                        }
                        self.tail(tail)?;
                        Ok(())
                    })
                })
                .next(|| {
                    let _indent_guard = indent_guard.unwrap_or_else(|| self.begin_indent());
                    for (op, expr) in chain {
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        self.out.token_space(op.as_str())?;
                        self.expr(expr)?;
                    }
                    self.tail(tail)?;
                    Ok(())
                })
                .result()
        })
    }
}

/// Traverses the tree to collect a sequence of chained binary operations.
/// Traversal will only include binary operators of the same precedence.
fn collect_binary_expr_chain<'a>(
    left: &'a ast::Expr,
    right: &'a ast::Expr,
    top_op: Spanned<ast::BinOpKind>,
) -> (&'a ast::Expr, Vec<(ast::BinOpKind, &'a ast::Expr)>) {
    let mut current = left;
    let mut stack = vec![right];
    let mut operators = vec![top_op.node];

    let (mut first, mut chain) = (None, Vec::new());

    loop {
        if let ast::ExprKind::Binary(op, ref left, ref right) = current.kind
            && op.node.precedence() == top_op.node.precedence()
        {
            current = left;
            operators.push(op.node);
            stack.push(right);
        } else {
            if first.is_none() {
                first = Some(current);
            } else {
                let operator = operators.pop().unwrap();
                chain.push((operator, current));
            }
            let Some(expr) = stack.pop() else {
                break;
            };
            current = expr;
        }
    }
    (first.unwrap(), chain)
}
