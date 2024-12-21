use rustc_ast::ast;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::error::FormatResult;

impl AstFormatter {
    pub fn match_(
        &self,
        scrutinee: &ast::Expr,
        arms: &[ast::Arm],
        expr: &ast::Expr,
        end: Tail<'_>,
    ) -> FormatResult {
        self.out.token_at("match", expr.span.lo())?;
        self.out.space()?;
        self.expr_tail(scrutinee, Tail::OPEN_BLOCK)?;
        self.block_generic_after_open_brace(arms, |arm| self.arm(arm))?;
        self.tail(end)
    }

    fn arm(&self, arm: &ast::Arm) -> FormatResult {
        self.attrs(&arm.attrs)?;
        self.pat(&arm.pat)?;
        let arrow = || {
            self.out.space()?;
            self.out.token_expect("=>")?;
            Ok(())
        };
        let broken_guard = if let Some(guard) = arm.guard.as_deref() {
            let broken = self.line_break_indent_fallback(|broken| {
                self.out.token_expect("if")?;
                self.out.space()?;
                self.expr_tail(
                    guard,
                    Tail::new(&|| {
                        arrow()?;
                        if !broken || self.config.rustfmt_quirks {
                            self.out.require_width(" {".len())?;
                        }
                        Ok(())
                    }),
                )?;
                Ok(broken)
            })?;
            if broken {
                self.out.newline_indent()?;
            }
            broken
        } else {
            false
        };
        if let Some(body) = arm.body.as_deref() {
            if arm.guard.is_none() {
                arrow()?;
                self.out.space()?;
                self.expr(body)?;
            } else if broken_guard {
                self.expr_force_block(body)?;
            } else {
                self.expr(body)?;
            }
            if is_plain_block(body) {
                self.out.skip_token_if_present(",")?;
            } else {
                self.out.token_expect(",")?;
            }
        } else {
            todo!();
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
