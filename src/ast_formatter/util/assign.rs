use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::tail::Tail;
use crate::ast_formatter::util::simulate_wrap::SimulateWrapResult;
use crate::error::FormatResult;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;

impl AstFormatter {
    pub fn assign_expr(&self, expr: &ast::Expr, tail: Tail) -> FormatResult {
        self.out.space_token("=")?;
        let checkpoint_after_eq = self.out.checkpoint();

        let force_wrap = if self.out.with_recover_width(|| self.out.space()).is_err() {
            true
        } else {
            match self.simulate_wrap_indent(0, || self.expr_tail(expr, tail))? {
                SimulateWrapResult::Ok => return Ok(()),
                SimulateWrapResult::NoWrap | SimulateWrapResult::WrapForLongerFirstLine => false,
                SimulateWrapResult::WrapForSingleLine
                | SimulateWrapResult::WrapForLessExcessWidth => true,
            }
        };

        self.out.restore_checkpoint(&checkpoint_after_eq);
        self.backtrack()
            .next_if(!force_wrap, |_| {
                self.space_could_wrap_indent(|| {
                    self.expr(expr)?;
                    self.tail(tail)?;
                    Ok(())
                })
            })
            .next(|_| {
                self.indented(|| {
                    self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    self.expr_tail(expr, tail)?;
                    Ok(())
                })
            })
            .result_with_checkpoint(&checkpoint_after_eq)?;
        Ok(())
    }
}
