use rustc_ast::ast;
use std::ops::ControlFlow;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::constraint_modifiers::INDENT_WIDTH;
use crate::ast_formatter::fallback::Fallback;
use crate::ast_formatter::util::tail::Tail;
use crate::ast_utils::{arm_body_requires_block, is_plain_block};
use crate::error::{ConstraintError, FormatError, FormatResult};
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
            self.arm_body(body)?;
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
            self.arm_body(body)?;
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
            self.arm_body_force_block(body)?;
        }
        Ok(())
    }

    fn arm_guard(&self, guard: &ast::Expr) -> FormatResult {
        self.out.token_space("if")?;
        self.expr(guard)?;
        Ok(())
    }

    fn arm_body(&self, body: &ast::Expr) -> FormatResult {
        if arm_body_requires_block(body) {
            self.expr_add_block(body)?;
        } else {
            self.skip_single_expr_blocks(body, |body| {
                if is_plain_block(body) {
                    self.expr(body)
                } else {
                    self.arm_body_maybe_add_block(body)
                }
            })?;
        }
        self.out.skip_token_if_present(",")?;
        Ok(())
    }

    fn arm_body_force_block(&self, body: &ast::Expr) -> FormatResult {
        if is_plain_block(body) {
            self.expr(body)?;
        } else {
            self.expr_add_block(body)?;
        }
        self.out.skip_token_if_present(",")?;
        Ok(())
    }

    fn arm_body_maybe_add_block(&self, body: &ast::Expr) -> FormatResult {
        // todo share logic with local which also wraps to avoid multi-line
        // todo should we count lines or simply observe whether it's multi-line?
        self.expr_add_block_if_first_line_is_longer(body)
    }

    /// Call this function with a fallback that will format the code on the next line and indented.
    /// This function will return `Err(WidthLimitExceeded)` if it can prove that the fallback will
    /// allow more code to fit in the first line of the output.
    fn expr_add_block_if_first_line_is_longer(&self, body: &ast::Expr) -> FormatResult {
        let add_block = || self.expr_add_block(body);
        let same_line_or_add_block = |fallback: Fallback| {
            // todo closures and structs should have single-line headers
            // todo exclude comma for block-like expressions?
            fallback
                .next(|| {
                    self.with_touchy_margins(|| self.expr_tail(body, &Tail::token_insert(",")))
                })
                .otherwise(add_block)
        };
        let Some(max_width) = self.constraints().max_width.get() else {
            return same_line_or_add_block(self.start_fallback());
        };

        let start = self.out.last_line_len();
        let next_line_start = self.constraints().indent.get() + INDENT_WIDTH;
        if start <= next_line_start {
            // wrap-indent wouldn't afford us more width so just continue normally
            return same_line_or_add_block(self.start_fallback());
        }
        let extra_width = start - next_line_start;

        // let checkpoint = self.checkpoint();
        // Try to format allowing the extra width that we would have.
        // Use single-line to limit the experiment to the first line
        // Also we are not applying touchy_margins here to simulate being in a block
        let result = self.start_fallback().next_control_flow(|| {
            let result = self.with_single_line(|| {
                self.constraints()
                    .max_width
                    .with_replaced(Some(max_width + extra_width), || self.expr(body))
            });
            let used_extra_width = self.out.last_line_len() > max_width;
            match (used_extra_width, result) {
                (
                    true,
                    Ok(()) | Err(FormatError::Constraint(ConstraintError::NewlineNotAllowed)),
                ) => ControlFlow::Continue(false),
                (false, Err(FormatError::Constraint(ConstraintError::NewlineNotAllowed))) => {
                    // the first line fits without extra width;
                    // try again without the single-line and extra width
                    ControlFlow::Continue(true)
                }
                (false, Ok(())) => {
                    // it fits on one line, but now we need a comma
                    match self.out.token_insert(",") {
                        Err(FormatError::Constraint(_)) => ControlFlow::Continue(false),
                        result => ControlFlow::Break(result),
                    }
                }
                (_, Err(e)) => ControlFlow::Break(Err(e)),
            }
        });
        match result {
            ControlFlow::Break(result) => result,
            ControlFlow::Continue((fallback, should_try_without_block)) => {
                if should_try_without_block {
                    same_line_or_add_block(fallback)
                } else {
                    fallback.otherwise(add_block)
                }
            }
        }
    }
}
