use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::tail::Tail;
use crate::error::FormatResult;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;

impl AstFormatter {
    pub fn expr_infix(
        &self,
        left: &ast::Expr,
        op: &'static str,
        right: &ast::Expr,
        tail: Tail,
    ) -> FormatResult {
        self.expr_tail(
            left,
            Some(&self.tail_fn(|af| {
                af.out.space_token_space(op)?;
                af.expr_tail(right, tail)?;
                Ok(())
            })),
        )
    }

    pub fn simple_infix_chain<T>(
        &self,
        token: &'static str,
        items: &[T],
        format_item: impl Fn(&T) -> FormatResult,
        should_indent: bool,
        tail: Tail,
    ) -> FormatResult {
        let (first, rest) = items.split_first().unwrap();
        self.backtrack()
            .next(|| {
                self.with_single_line(|| {
                    format_item(first)?;
                    for item in rest {
                        self.out.space_token_space(token)?;
                        format_item(item)?;
                    }
                    self.tail(tail)?;
                    Ok(())
                })
            })
            .next(|| {
                format_item(first)?;
                self.indented_optional(should_indent, || {
                    for item in rest {
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        self.out.token_space(token)?;
                        format_item(item)?;
                    }
                    self.tail(tail)?;
                    Ok(())
                })?;
                Ok(())
            })
            .result()
    }
}
