use rustc_ast::ast;

use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;

impl AstFormatter {
    pub fn match_(
        &self,
        scrutinee: &ast::Expr,
        arms: &[ast::Arm],
    ) -> FormatResult {
        self.token_expr_open_brace("match", scrutinee)?;
        self.block_generic_after_open_brace(arms, |arm| self.arm(arm))?;
        Ok(())
    }

    fn arm(&self, arm: &ast::Arm) -> FormatResult {
        self.attrs(&arm.attrs)?;
        let first_line = self.out.line();
        self.pat(&arm.pat)?;
        let arrow = || -> FormatResult {
            self.out.space()?;
            self.out.token_expect("=>")?;
            Ok(())
        };
        let comma = |body| {
            if is_plain_block(body) {
                self.out.skip_token_if_present(",")
            } else {
                self.out.token_expect(",")
            }
        };
        if let Some(guard) = arm.guard.as_deref() {
            let if_guard = || -> FormatResult {
                self.out.token_expect("if")?;
                self.out.space()?;
                self.expr(guard)?;
                Ok(())
            };
            let guard_same_line = || {
                self.with_single_line(|| -> FormatResult {
                    self.out.space()?;
                    if_guard()?;
                    Ok(())
                })?;
                if let Some(body) = arm.body.as_deref() {
                    arrow()?;
                    self.out.space()?;
                    self.fallback(|| self.expr(body))
                        .next(|| self.expr_force_block(body))
                        .result()?;
                    comma(body)?;
                }
                Ok(())
            };
            let guard_separate_line = || {
                self.indented(|| {
                    self.out.newline_indent()?;
                    if_guard()?;
                    Ok(())
                })?;
                if let Some(body) = arm.body.as_deref() {
                    arrow()?;
                    if self.config.rustfmt_quirks {
                        self.out.require_width(" {".len())?;
                    }
                    self.out.newline_indent()?;
                    self.expr_force_block(body)?;
                    comma(body)?;
                }
                Ok(())
            };
            if self.out.line() == first_line {
                self.fallback(guard_same_line)
                    .next(guard_separate_line)
                    .result()?;
            } else {
                guard_separate_line()?;
            }
        } else if let Some(body) = arm.body.as_deref() {
            arrow()?;
            self.out.space()?;
            self.expr(body)?;
            comma(body)?;
        }
        Ok(())
    }
}

fn is_plain_block(expr: &ast::Expr) -> bool {
    match &expr.kind {
        ast::ExprKind::Block(block, None) => matches!(block.rules, ast::BlockCheckMode::Default),
        _ => false,
    }
}
