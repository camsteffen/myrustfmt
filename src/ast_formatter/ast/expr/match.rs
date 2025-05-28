use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::simulate_wrap::SimulateWrapResult;
use crate::ast_utils::{arm_body_requires_block, plain_block};
use crate::constraints::VStruct;
use crate::error::FormatResult;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;

impl AstFormatter {
    pub fn match_(&self, scrutinee: &ast::Expr, arms: &[ast::Arm]) -> FormatResult {
        self.has_vstruct(VStruct::Match, || {
            self.control_flow_header("match", scrutinee)?;
            self.block(true, arms, |arm| self.arm(arm))?;
            Ok(())
        })
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
                    self.out.with_recover_width(|| self.arm_guard_same_line(arm, guard))
                })
                .next(|| self.arm_guard_separate_line(arm, guard))
                .result()?;
        } else if let Some(body) = arm.body.as_deref() {
            self.pat_tail(
                &arm.pat,
                self.tail_fn(|af| {
                    af.out.space_token_space("=>")?;
                    af.arm_body(body)?;
                    Ok(())
                })
                .as_ref(),
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
        let Some(body) = arm.body.as_deref() else {
            return Ok(());
        };
        self.out.space_token("=>")?;
        if plain_block(body)
            .is_some_and(|block| self.is_block_empty(block))
        {
            self.out.space()?;
            self.expr(body)?;
            return Ok(());
        }
        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
        self.expr_force_plain_block(body)?;
        self.out.skip_token_if_present(",")?;
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

    // todo share logic with local which also wraps to avoid multi-line
    // todo should we count lines or simply observe whether it's multi-line?
    fn arm_body_maybe_add_block(&self, body: &ast::Expr) -> FormatResult {
        let checkpoint = self.out.checkpoint();
        let (force_block, lookahead) = match self.simulate_wrap_indent(true, || self.expr(body)) {
            SimulateWrapResult::Wrap { single_line } => (
                true,
                single_line.then(|| self.out.capture_lookahead(&checkpoint)),
            ),
            SimulateWrapResult::NoWrap => (false, None),
            SimulateWrapResult::Ok => {
                if self
                    .out
                    .with_recover_width(|| self.out.token_insert(","))
                    .is_err()
                {
                    (true, None)
                } else {
                    return Ok(());
                }
            }
        };
        if lookahead.is_none() {
            self.out.restore_checkpoint(&checkpoint);
        }
        // todo exclude comma for block-like expressions?
        self.backtrack_from_checkpoint(checkpoint)
            .next_if(!force_block, || {
                self.disallow_vstructs(VStruct::BrokenIndent | VStruct::HangingIndent, || {
                    self.expr_tail(body, self.tail_fn(|af| af.out.token_insert(",")).as_ref())
                })
            })
            .next(|| {
                self.add_block(|| {
                    if let Some(lookahead) = lookahead {
                        self.out.restore_lookahead(lookahead);
                    } else {
                        self.expr(body)?;
                    }
                    Ok(())
                })
            })
            .result()?;
        Ok(())
    }
}
