use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::tail::Tail;
use crate::error::FormatResult;

use crate::constraints::MultiLineShape;
use rustc_ast::ast;
use rustc_ast::util::parser::AssocOp;
use rustc_span::source_map::Spanned;

impl AstFormatter {
    pub fn binary_expr(
        &self,
        left: &ast::Expr,
        right: &ast::Expr,
        op: Spanned<ast::BinOpKind>,
        tail: &Tail,
    ) -> FormatResult {
        let (first, chain) = collect_binary_expr_chain(left, right, op);
        let first_line = self.out.line();
        self.expr(first)?;
        self.backtrack()
            // format all on one line, only if the first item fits in one line
            .next_if(self.out.line() == first_line, || {
                self.with_single_line(|| {
                    for (op, expr) in &chain {
                        self.out.space_token_space(op.as_str())?;
                        self.expr(expr)?;
                    }
                    self.tail(tail)?;
                    Ok(())
                })
            })
            .otherwise(|| {
                self.constraints()
                    .with_single_line_unless(MultiLineShape::HangingIndent, || {
                        let mut iter = chain.iter();
                        let mut within_margin = self.take_while_within_margin(iter.by_ref());
                        loop {
                            let Some((op, expr)) = within_margin.next() else {
                                drop(within_margin);
                                break;
                            };
                            let success = self
                                .backtrack()
                                .next(|| {
                                    self.out.space_token_space(op.as_str())?;
                                    self.expr(expr)?;
                                    Ok(true)
                                })
                                .otherwise(|| Ok(false))?;
                            if !success {
                                drop(within_margin);
                                // back up one for a redo
                                let index = chain.len() - iter.len() - 1;
                                iter = chain[index..].iter();
                                break;
                            }
                        }
                        if iter.as_slice().is_empty() {
                            // don't indent the tail in this case
                            return self.tail(tail);
                        }
                        self.indented(|| {
                            for (op, expr) in iter {
                                self.newline_break_indent()?;
                                self.out.token_space(op.as_str())?;
                                self.expr(expr)?;
                            }
                            self.tail(tail)?;
                            Ok(())
                        })?;
                        Ok(())
                    })
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
    let mut first = None;
    let mut chain = Vec::new();
    let mut stack = vec![right];
    let mut operators = vec![top_op.node];
    let precedence = AssocOp::from_ast_binop(top_op.node).precedence();
    let mut current = left;

    loop {
        match current.kind {
            ast::ExprKind::Binary(op, ref left, ref right)
                if AssocOp::from_ast_binop(op.node).precedence() == precedence =>
            {
                operators.push(op.node);
                current = left;
                stack.push(right);
            }
            _ => {
                if first.is_none() {
                    first = Some(current);
                } else {
                    let op = operators.pop().unwrap();
                    chain.push((op, current));
                }
                match stack.pop() {
                    None => break,
                    Some(expr) => current = expr,
                }
            }
        }
    }
    (first.unwrap(), chain)
}
