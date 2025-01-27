use rustc_ast::ast;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::tail::Tail;
use crate::ast_utils::{arm_body_requires_block, is_plain_block};
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
        if let Some(guard) = arm.guard.as_deref() {
            if self.out.line() == first_line {
                self.fallback(|| self.arm_guard_same_line(arm, guard))
                    .otherwise(|| self.arm_guard_separate_line(arm, guard))?;
            } else {
                self.arm_guard_separate_line(arm, guard)?;
            }
        } else if let Some(body) = arm.body.as_deref() {
            self.out.space_token_space("=>")?;
            self.arm_body(body, false)?;
        }
        Ok(())
    }

    fn arm_guard_same_line(&self, arm: &ast::Arm, guard: &ast::Expr) -> FormatResult {
        self.with_single_line(|| -> FormatResult {
            self.out.space()?;
            self.arm_guard(guard)?;
            Ok(())
        })?;
        if let Some(body) = arm.body.as_deref() {
            self.out.space_token_space("=>")?;
            self.arm_body(body, false)?;
        }
        Ok(())
    }

    fn arm_guard_separate_line(&self, arm: &ast::Arm, guard: &ast::Expr) -> FormatResult {
        self.indented(|| {
            self.out.newline_within_indent()?;
            self.arm_guard(guard)?;
            Ok(())
        })?;
        if let Some(body) = arm.body.as_deref() {
            self.out.space_token("=>")?;
            self.out.newline_within_indent()?;
            // todo allow single line without block?
            self.arm_body(body, true)?;
        }
        Ok(())
    }

    fn arm_guard(&self, guard: &ast::Expr) -> FormatResult {
        self.out.token_space("if")?;
        self.expr(guard)?;
        Ok(())
    }

    fn arm_body(&self, body: &ast::Expr, force_block: bool) -> FormatResult {
        if force_block {
            if is_plain_block(body) {
                self.expr(body)?;
            } else {
                self.expr_add_block(body)?;
            }
        } else if arm_body_requires_block(body) {
            self.expr_add_block(body)?;
        } else {
            self.skip_single_expr_blocks(body, |body| {
                if is_plain_block(body)  {
                    self.expr(body)
                } else {
                    self.fallback(|| {
                        // todo closures and structs should have single-line headers
                        // todo exclude comma for block-like expressions?
                        self.with_touchy_margins(|| self.expr_tail(body, &Tail::token_insert(",")))
                    })
                    .otherwise(|| self.expr_add_block(body))
                }
            })?;
        }
        self.out.skip_token_if_present(",")?;
        Ok(())
    }
}
