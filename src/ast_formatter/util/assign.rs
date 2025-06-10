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

        let (force_wrap, lookahead) = if self.out.with_recover_width(|| self.out.space()).is_err() {
            (true, None)
        } else {
            let checkpoint_after_space = self.out.checkpoint();
            match self.simulate_wrap_indent(|| self.expr_tail(expr, tail)) {
                SimulateWrapResult::Ok => return Ok(()),
                SimulateWrapResult::NoWrap | SimulateWrapResult::WrapForLongerFirstLine => (false, None),
                SimulateWrapResult::WrapForLessExcessWidth => (true, None),
                SimulateWrapResult::WrapForSingleLine => (true, Some(self.out.capture_lookahead(&checkpoint_after_space))),
            }
        };

        self.backtrack()
            .next_if(!force_wrap, || {
                self.space_could_wrap_indent(|| {
                    self.expr(expr)?;
                    self.tail(tail)?;
                    Ok(())
                })
            })
            .next(|| {
                self.indented(|| {
                    self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    if let Some(lookahead) = lookahead {
                        self.out.restore_lookahead(lookahead);
                    } else {
                        self.expr(expr)?;
                        self.tail(tail)?;
                    }
                    Ok(())
                })
            })
            .result_with_checkpoint(&checkpoint_after_eq, true)?;
        Ok(())
    }
}
