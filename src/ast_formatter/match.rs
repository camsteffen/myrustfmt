use rustc_ast::ast;
use std::ops::ControlFlow;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::backtrack::Backtrack;
use crate::ast_formatter::constraint_modifiers::INDENT_WIDTH;
use crate::ast_formatter::util::tail::Tail;
use crate::ast_utils::{arm_body_requires_block, plain_block};
use crate::error::{ConstraintError, FormatError, FormatResult, return_if_break};
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
        if let Some(guard) = arm.guard.as_deref() {
            self.pat(&arm.pat)?;
            if self.out.line() == first_line {
                self.backtrack()
                    .next(|| self.arm_guard_same_line(arm, guard))
                    .otherwise(|| self.arm_guard_separate_line(arm, guard))?;
            } else {
                self.arm_guard_separate_line(arm, guard)?;
            }
        } else if let Some(body) = arm.body.as_deref() {
            self.pat_tail(
                &arm.pat,
                &Tail::func(|af| {
                    af.out.space_token_space("=>")?;
                    af.arm_body(body)?;
                    Ok(())
                }),
            )?;
        }
        Ok(())
    }

    fn arm_guard_same_line(&self, arm: &ast::Arm, guard: &ast::Expr) -> FormatResult {
        self.with_single_line(|| {
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
        self.skip_single_expr_blocks(body, |body| if arm_body_requires_block(body) {
            self.expr_add_block(body)
        } else if plain_block(body).is_some() {
            self.expr(body)
        } else {
            self.arm_body_add_block_if_first_line_is_longer(body)
        })?;
        self.out.skip_token_if_present(",")?;
        Ok(())
    }

    fn arm_body_force_block(&self, body: &ast::Expr) -> FormatResult {
        if let Some(block) = plain_block(body) {
            self.block_separate_lines(block)?;
        } else {
            self.expr_add_block(body)?;
        }
        self.out.skip_token_if_present(",")?;
        Ok(())
    }

    // todo share logic with local which also wraps to avoid multi-line
    // todo should we count lines or simply observe whether it's multi-line?
    /// Adds a block only if doing so allows for more code to fit in the first line
    fn arm_body_add_block_if_first_line_is_longer(&self, body: &ast::Expr) -> FormatResult {
        let Some(max_width) = self.constraints().max_width.get() else {
            return self.arm_body_same_line(body, self.backtrack());
        };

        let start = self.out.last_line_len();
        // the starting position if we wrapped to the next line and indented
        let next_line_start = self.constraints().indent.get() + INDENT_WIDTH;
        if start <= next_line_start {
            // wrap-indent wouldn't afford us more width so just continue normally
            return self.arm_body_same_line(body, self.backtrack());
        }
        let extra_width = start - next_line_start;

        let result = self.backtrack().next_control_flow(|| {
            // We're going to try formatting on the same line, but adding extra width to simulate
            // wrapping with a block. Use the single-line constraint since we just want to see what
            // fits on the first line.
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
                ) => {
                    // we used the extra width, so we need to add a block to make the first line fit
                    ControlFlow::Continue(true)
                }
                (false, Err(FormatError::Constraint(ConstraintError::NewlineNotAllowed))) => {
                    // we did not use the extra width, but it did not fit on one line,
                    // so try to format normally without a block
                    ControlFlow::Continue(false)
                }
                (false, Ok(())) => {
                    // it fits on one line, but now we need a comma
                    match self.out.token_insert(",") {
                        // welp the comma didn't fit,
                        // but the expression will fit on one line if we add a block
                        Err(FormatError::Constraint(_)) => ControlFlow::Continue(true),
                        // it all fits on one line!
                        Ok(()) => ControlFlow::Break(Ok(())),
                        // terminal error
                        Err(e) => ControlFlow::Break(Err(e)),
                    }
                }
                // terminal error
                (_, Err(e)) => ControlFlow::Break(Err(e)),
            }
        });
        let (backtrack, should_add_block) = return_if_break!(result);
        if should_add_block {
            backtrack.otherwise(|| self.expr_add_block(body))
        } else {
            self.arm_body_same_line(body, backtrack)
        }
    }

    fn arm_body_same_line(&self, body: &ast::Expr, backtrack: Backtrack) -> FormatResult {
        // todo closures and structs should have single-line headers
        // todo exclude comma for block-like expressions?
        backtrack
            .next(|| {
                self.constraints()
                    .with_single_line_chains(|| self.expr_tail(body, &Tail::token_insert(",")))
            })
            .otherwise(|| self.expr_add_block(body))
    }
}
