use rustc_ast::ast;

use crate::ast_formatter::AstFormatter;
use crate::ast_utils::is_plain_block;
use crate::error::FormatResult;

impl AstFormatter {
    pub fn match_(&self, scrutinee: &ast::Expr, arms: &[ast::Arm]) -> FormatResult {
        self.token_expr_open_brace("match", scrutinee)?;
        self.block_generic_after_open_brace(arms, |arm| self.arm(arm))?;
        Ok(())
    }

    fn arm(&self, arm: &ast::Arm) -> FormatResult {
        self.with_attrs(&arm.attrs, arm.span, || self.arm_after_attrs(arm))
    }
    
    fn arm_after_attrs(&self, arm: &ast::Arm) -> FormatResult {
        let first_line = self.out.line();
        self.pat(&arm.pat)?;
        let comma = |body| {
            if is_plain_block(body) {
                self.out.skip_token_if_present(",")
            } else {
                self.out.token(",")
            }
        };
        if let Some(guard) = arm.guard.as_deref() {
            let if_guard = || -> FormatResult {
                self.out.token_space("if")?;
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
                    self.out.space_token("=>")?;
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
                    self.out.space_token("=>")?;
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
            self.out.space_token_space("=>")?;
            self.expr(body)?;
            comma(body)?;
        }
        Ok(())
    }
}
