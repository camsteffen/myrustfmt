use rustc_ast::ast;

use crate::ast_formatter::AstFormatter;
use crate::ast_utils::{arm_body_requires_block, plain_block};
use crate::constraints::VerticalShape;
use crate::error::FormatResult;
use crate::whitespace::VerticalWhitespaceMode;

impl AstFormatter {
    pub fn match_(&self, scrutinee: &ast::Expr, arms: &[ast::Arm]) -> FormatResult {
        self.token_expr_open_brace("match", scrutinee)?;
        self.block_after_open_brace(arms, |arm| self.arm(arm))?;
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
            self.out.newline_indent(VerticalWhitespaceMode::Break)?;
            self.arm_guard(guard)?;
            Ok(())
        })?;
        if let Some(body) = arm.body.as_deref() {
            self.out.space_token("=>")?;
            if plain_block(body)
                .is_some_and(|block| self.is_block_empty(block))
            {
                self.out.space()?;
                self.expr(body)?;
            } else {
                self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                // todo allow single line without block?
                self.arm_body_force_block(body)?;
            }
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
                self.arm_body_maybe_add_block(body)
            }
        })?;
        self.out.skip_token_if_present(",")?;
        Ok(())
    }

    fn arm_body_force_block(&self, body: &ast::Expr) -> FormatResult {
        if let Some(block) = plain_block(body) {
            self.block_expr(block)?;
        } else {
            self.expr_add_block(body)?;
        }
        self.out.skip_token_if_present(",")?;
        Ok(())
    }

    // todo share logic with local which also wraps to avoid multi-line
    // todo should we count lines or simply observe whether it's multi-line?
    fn arm_body_maybe_add_block(&self, body: &ast::Expr) -> FormatResult {
        let checkpoint = self.out.checkpoint();
        enum Next {
            Done,
            AddBlock,
            Normal,
        }
        let result = self.out.with_enforce_max_width(|| {
            // simulate having extra width if we had added a block
            let (used_extra_width, result) =
                self.simulate_wrap_indent_first_line(|| self.expr(body));
            if used_extra_width {
                // Formatting on the same line _might_ be possible,
                // but adding a block allows for a longer first line.
                return Next::AddBlock;
            }
            if result.is_err() {
                // We did not use the extra width, and it did not fit on one line,
                // so try again to format on the same line without the extra width.
                return Next::Normal;
            }
            // it fits on one line, but now we need a comma (if we're not adding a block)
            if self.out.token_insert(",").is_err() {
                return Next::AddBlock;
            }
            Next::Done
        });
        match result {
            Next::Done => {}
            Next::AddBlock => {
                self.out.restore_checkpoint(&checkpoint);
                self.expr_add_block(body)?;
            }
            Next::Normal => {
                // todo closures and structs should have single-line headers
                // todo exclude comma for block-like expressions?
                self.backtrack_from_checkpoint(checkpoint)
                    .next(|| {
                        self.with_vertical_shape_min(VerticalShape::List, || {
                            self.expr_tail(body, &self.tail_fn(|af| af.out.token_insert(",")))
                        })
                    })
                    .otherwise(|| self.expr_add_block(body))?
            }
        }
        Ok(())
    }
}
