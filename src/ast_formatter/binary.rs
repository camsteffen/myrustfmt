use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;

use rustc_ast::ast;
use rustc_ast::util::parser::AssocOp;
use rustc_span::source_map::Spanned;

impl AstFormatter {
    pub fn binary(
        &self,
        left: &ast::Expr,
        right: &ast::Expr,
        op: Spanned<ast::BinOpKind>,
        tail: &Tail,
    ) -> FormatResult {
        let (first, chain) = self.collect_binary_chain(left, right, op);
        self.expr(first)?;
        self.fallback(|| {
            self.with_single_line(|| {
                chain.iter().try_for_each(|(op, expr)| -> FormatResult {
                    self.out.space_token_space(op.as_str())?;
                    self.expr(expr)?;
                    Ok(())
                })?;
                self.tail(tail)?;
                Ok(())
            })
        })
        .otherwise(|| {
            self.indented(|| {
                for (op, expr) in chain {
                    self.out.newline_indent()?;
                    self.out.token_space(op.as_str())?;
                    self.expr(expr)?;
                }
                self.tail(tail)?;
                Ok(())
            })
        })
    }

    fn collect_binary_chain<'a>(
        &self,
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
                        Some(expr) => {
                            current = expr;
                        }
                    }
                }
            }
        }
        (first.unwrap(), chain)
    }
}
