use rustc_ast::ast;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::tail::Tail;
use crate::ast_utils::{arm_body_requires_block, plain_block};
use crate::error::FormatResult;
use crate::util::cell_ext::CellExt;

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
        self.skip_single_expr_blocks(body, |body| {
            let with_add_block = || {
                self.add_block(|| self.expr(body))?;
                self.out.skip_token_if_present(",")?;
                Ok(())
            };
            if force_block || arm_body_requires_block(body) {
                with_add_block()
            } else {
                self.fallback(|| {
                    // todo should be block-like?
                    let tail = if plain_block(body).is_some() {
                        Tail::token_skip_if_present(",")
                    } else {
                        Tail::token_maybe_missing(",")
                    };
                    self.out
                        .constraints()
                        .touchy_margin
                        .with_replaced(true, || self.expr_tail(body, &tail))?;
                    Ok(())
                })
                .otherwise(with_add_block)
            }
        })
    }
}
