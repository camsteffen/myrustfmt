use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::tail::Tail;
use crate::ast_formatter::util::simulate_wrap::SimulateWrapDecision;
use crate::error::FormatResult;
use crate::num::HSize;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;

impl AstFormatter {
    pub fn local(&self, local: &ast::Local) -> FormatResult {
        self.with_attrs(&local.attrs, local.span, || self.local_after_attrs(local))
    }

    fn local_after_attrs(&self, local: &ast::Local) -> FormatResult {
        let ast::Local { pat, kind, ty, .. } = local;
        let (first_line, start_col) = self.out.line_col();
        self.out.token_space("let")?;
        self.pat_tail(
            pat,
            &(ty.is_none() && kind.init().is_none())
                .then_some(self.tail_token_inner(";")),
        )?;
        if let Some(ty) = ty {
            self.out.token_space(":")?;
            // todo tail with expression?
            self.ty_tail(
                ty,
                &kind.init().is_none().then_some(self.tail_token_inner(";")),
            )?;
        }
        let Some((init, else_)) = kind.init_else_opt() else {
            return Ok(());
        };
        self.local_init(init, &else_.is_none().then_some(self.tail_token_inner(";")))?;
        let Some(else_) = else_ else { return Ok(()) };
        let is_single_line_init = self.out.line() == first_line;
        self.local_else(else_, is_single_line_init, start_col)?;
        Ok(())
    }

    fn local_init(&self, expr: &ast::Expr, tail: Tail) -> FormatResult {
        self.out.space_token("=")?;
        let checkpoint_after_eq = self.out.checkpoint();

        let (try_same_line, lookahead) = if self
            .out
            .with_recoverable_width(|| self.out.space())
            .is_err()
        {
            (false, None)
        } else {
            let simulate_wrap_result =
                self.simulate_wrap_indent_first_line(false, || self.expr_tail(expr, tail));
            match simulate_wrap_result {
                SimulateWrapDecision::SameLine => (true, None),
                SimulateWrapDecision::Keep => return Ok(()),
                SimulateWrapDecision::Wrap { single_line } => (false, single_line),
            }
        };

        self.out.restore_checkpoint(&checkpoint_after_eq);
        self.backtrack_from_checkpoint(checkpoint_after_eq)
            .next_if(try_same_line, || {
                self.out.space()?;
                self.expr(expr)?;
                self.tail(tail)?;
                Ok(())
            })
            .otherwise(|| {
                self.indented(|| {
                    self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    if let Some(lookahead) = lookahead {
                        self.out.restore_lookahead(lookahead);
                    } else {
                        self.expr(expr)?;
                        self.tail(tail)?;
                    }
                    Ok(())
                })
            })?;
        Ok(())
    }

    fn local_else(
        &self,
        else_: &ast::Block,
        is_single_line_init: bool,
        start_col: HSize,
    ) -> FormatResult {
        let else_block_vertical = || {
            self.block_expr(true, else_)?;
            self.out.token(";")?;
            Ok(())
        };
        let same_line_else = || -> FormatResult {
            self.out.space_token_space("else")?;
            self.out.token("{")?;
            let else_block_horizontal = 'horizontal: {
                let expr_only_else = if is_single_line_init {
                    self.try_into_expr_only_block(else_)
                } else {
                    None
                };
                let Some(expr_only_else) = expr_only_else else {
                    break 'horizontal None;
                };
                Some(move || {
                    self.with_single_line(|| {
                        self.with_width_limit_from_start(
                            start_col,
                            RUSTFMT_CONFIG_DEFAULTS.single_line_let_else_max_width,
                            || {
                                self.expr_only_block_after_open_brace(expr_only_else)?;
                                self.out.token(";")?;
                                Ok(())
                            },
                        )
                    })
                })
            };
            self.backtrack()
                .next_opt(else_block_horizontal)
                .otherwise(else_block_vertical)?;
            Ok(())
        };
        let next_line_else = || -> FormatResult {
            self.out.newline_indent(VerticalWhitespaceMode::Break)?;
            self.out.token_space("else")?;
            self.out.token("{")?;
            else_block_vertical()?;
            Ok(())
        };
        self.backtrack()
            .next_if(
                is_single_line_init || self.out.last_line_is_closers(),
                same_line_else,
            )
            .otherwise(next_line_else)?;
        Ok(())
    }
}
