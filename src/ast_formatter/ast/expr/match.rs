use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::simulate_wrap::SimulateWrapResult;
use crate::ast_utils::plain_block;
use crate::constraints::VStruct;
use crate::error::FormatResult;
use crate::util::cell_ext::CellExt;
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
        self.with_attrs_tail(
            &arm.attrs,
            arm.span.into(),
            Some(&self.tail_fn(|af| {
                af.out.token_if_present(",")?;
                Ok(())
            })),
            || self.arm_after_attrs(arm),
        )?;
        // N.B. the span does not include the comma
        self.out.token_skip_if_present(",")?;
        Ok(())
    }

    fn arm_after_attrs(&self, arm: &ast::Arm) -> FormatResult {
        if let Some(guard) = arm.guard.as_deref() {
            self.arm_with_guard(arm, guard)?;
        } else if let Some(body) = arm.body.as_deref() {
            self.pat_tail(
                &arm.pat,
                Some(&self.tail_fn(|af| {
                    af.out.space_token_space("=>")?;
                    af.arm_body_maybe_remove_block(body)?;
                    Ok(())
                })),
            )?;
        }
        Ok(())
    }

    fn arm_with_guard(&self, arm: &ast::Arm, guard: &ast::Expr) -> FormatResult {
        let first_line = self.out.line();
        self.pat(&arm.pat)?;
        let single_line_arm_guard = || {
            {
                let _guard = self.single_line_guard();
                self.out.space()?;
                self.out.token_space("if")?;
                self.expr(guard)?;
            }
            if let Some(body) = arm.body.as_deref() {
                self.out.space_token_space("=>")?;
                self.arm_body_maybe_remove_block(body)?;
            }
            Ok(())
        };
        let next_line_arm_guard = || {
            let arm_indent = self.out.total_indent.get();
            let empty_body = arm.body.as_deref().and_then(plain_block).is_some_and(
                |block| self.is_block_empty(block),
            );
            self.indented(|| {
                self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                self.out.token_space("if")?;
                self.expr_tail(
                    guard,
                    arm.body
                        .as_deref()
                        .map(|body| {
                            self.tail_fn(|af| {
                                af.out.space_token("=>")?;
                                if empty_body {
                                    let _guard = self.out.total_indent.replace_guard(arm_indent);
                                    af.out.space_allow_newlines()?;
                                    af.expr(body)?;
                                }
                                Ok(())
                            })
                        })
                        .as_ref(),
                )?;
                Ok(())
            })?;
            if let Some(body) = arm.body.as_deref()
                && !empty_body
            {
                self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                self.expr_force_plain_block(body)?;
            }
            Ok(())
        };
        self.backtrack()
            .next_if(self.out.line() == first_line, |_| {
                self.could_wrap_indent(single_line_arm_guard)
            })
            .next(|_| next_line_arm_guard())
            .result()?;
        Ok(())
    }

    fn arm_body_maybe_remove_block(&self, body: &ast::Expr) -> FormatResult {
        self.skip_single_expr_blocks(body, |body| {
            if plain_block(body).is_some() {
                self.expr(body)
            } else {
                self.arm_body_maybe_add_block(body)
            }
        })
    }

    fn arm_body_maybe_add_block(&self, body: &ast::Expr) -> FormatResult {
        let checkpoint = self.out.checkpoint();
        let force_block = match self.simulate_wrap_indent(0, || self.expr(body))? {
            SimulateWrapResult::Ok => {
                let _guard = self.recover_width_guard();
                if self.out.token_maybe_missing(",").is_err() {
                    true
                } else {
                    return Ok(());
                }
            }
            SimulateWrapResult::NoWrap => false,
            SimulateWrapResult::WrapForSingleLine
            | SimulateWrapResult::WrapForLongerFirstLine
            | SimulateWrapResult::WrapForLessExcessWidth => true,
        };
        self.out.restore_checkpoint(&checkpoint);
        self.backtrack()
            .next_if(!force_block, |recover| {
                self.disallow_vstructs(
                    VStruct::ControlFlow | VStruct::NonBlockIndent,
                    recover,
                    || self.expr_tail(body, Some(&self.tail_fn(|af| af.out.token_insert(",")))),
                )
            })
            .next(|_| self.add_block(|| self.expr_stmt(body)))
            .result_with_checkpoint(&checkpoint)?;
        Ok(())
    }
}
