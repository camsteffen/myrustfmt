use rustc_ast::ast;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::backtrack::Backtrack;
use crate::ast_formatter::util::constraint_modifiers::INDENT_WIDTH;
use crate::ast_utils::{arm_body_requires_block, plain_block};
use crate::constraints::MultiLineShape;
use crate::error::{ConstraintErrorKind, FormatResult, FormatResultExt};

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
            self.backtrack()
                .next_if(self.out.line() == first_line, || {
                    self.arm_guard_same_line(arm, guard)
                })
                .otherwise(|| self.arm_guard_separate_line(arm, guard))?;
        } else if let Some(body) = arm.body.as_deref() {
            self.pat_tail(
                &arm.pat,
                &self.tail_fn(|af| {
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
        self.skip_single_expr_blocks(body, |body| {
            if arm_body_requires_block(body) {
                self.expr_add_block(body)
            } else if plain_block(body).is_some() {
                self.expr(body)
            } else {
                self.arm_body_add_block_if_first_line_is_longer(body)
            }
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
        let Some(max_width) = self.constraints().borrow().max_width else {
            return self.arm_body_same_line(body, self.backtrack());
        };

        let start = self.out.last_line_len();
        // the starting position if we wrapped to the next line and indented
        let next_line_start = self.out.indent.get() + INDENT_WIDTH;
        if start <= next_line_start {
            // adding a block wouldn't afford us more width so no need to experiment to see if it
            // would be fewer lines
            // todo is this even possible?
            return self.arm_body_same_line(body, self.backtrack());
        }
        let extra_width = start - next_line_start;

        let checkpoint = self.open_checkpoint();
        // We're going to try formatting on the same line, but adding the extra width we would have
        // if wrapping with a block. Use the single-line constraint since we just want to see what
        // fits on the first line.
        let result = self
            .with_single_line(|| {
                self.constraints()
                    .with_max_width(Some(max_width + extra_width), || self.expr(body))
            })
            .constraint_err_only()?;
        let succeeded = match result {
            Err(e) if e.kind != ConstraintErrorKind::NewlineNotAllowed => {
                return Err(e.into());
            }
            result => result.is_ok(),
        };
        let used_extra_width = self.out.last_line_len() > max_width;
        let should_add_block = if used_extra_width {
            // We used the extra width, so we will add a block to make the first line fit.
            // (A block may not be strictly needed, but without a block, the result would be broken
            // into more lines.)
            true
        } else if !succeeded {
            // we did not use the extra width, but it did not fit on one line,
            // so try to format normally without a block
            false
        } else {
            // it fits on one line, but now we need a comma
            match self.out.token_insert(",").constraint_err_only()? {
                // welp the comma didn't fit,
                // but the expression will fit on one line if we add a block
                Err(_) => true,
                // it all fits on one line!
                Ok(()) => return Ok(()),
            }
        };
        self.restore_checkpoint(&checkpoint);
        if should_add_block {
            drop(checkpoint);
            self.expr_add_block(body)
        } else {
            self.arm_body_same_line(body, self.backtrack_from_checkpoint(checkpoint))
        }
    }

    fn arm_body_same_line(&self, body: &ast::Expr, backtrack: Backtrack) -> FormatResult {
        // todo closures and structs should have single-line headers
        // todo exclude comma for block-like expressions?
        backtrack
            .next(|| {
                self.constraints()
                    .with_multi_line_shape_min(MultiLineShape::VerticalList, || {
                        self.expr_tail(body, &self.tail_fn(|af| af.out.token_insert(",")))
                    })
            })
            .otherwise(|| self.expr_add_block(body))
    }
}
