use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::tail::Tail;
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use crate::source_formatter::Lookahead;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;

impl AstFormatter {
    pub fn local(&self, local: &ast::Local) -> FormatResult {
        self.with_attrs(&local.attrs, local.span, || self.local_after_attrs(local))
    }

    fn local_after_attrs(&self, local: &ast::Local) -> FormatResult {
        let ast::Local { pat, kind, ty, .. } = local;
        let first_line = self.out.line();
        let start = self.out.last_line_len();
        self.out.token_space("let")?;
        let Some((init, else_)) = kind.init_else_opt() else {
            let Some(ty) = ty else {
                self.pat_tail(pat, &self.tail_token(";"))?;
                return Ok(());
            };
            self.pat(pat)?;
            // todo tail?
            self.out.token_space(":")?;
            self.ty_tail(ty, &self.tail_token(";"))?;
            return Ok(());
        };

        // with initializer...

        self.pat(pat)?;
        if let Some(ty) = ty {
            // todo tail?
            self.out.token_space(":")?;
            self.ty(ty)?;
        }
        // else else else lol
        let Some(else_) = else_ else {
            self.local_init(init, &self.tail_token(";"))?;
            return Ok(());
        };

        // with else...

        self.local_init(init, Tail::none())?;
        let is_single_line_init = self.out.line() == first_line;
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
                            start,
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

    fn local_init(&self, expr: &ast::Expr, tail: &Tail) -> FormatResult {
        self.out.space_token("=")?;
        let checkpoint_after_eq = self.out.checkpoint();
        enum Next {
            SameLine,
            Wrap(Option<Lookahead>),
        }
        let next = if self
            .out
            .with_enforce_max_width(|| self.out.space())
            .is_err()
        {
            // comments forced a line break
            Next::Wrap(None)
        } else {
            let checkpoint_after_space = self.out.checkpoint();

            // simulate extra width from wrap-indent
            let (used_extra_width, result) = self.out.with_enforce_max_width(|| {
                self.simulate_wrap_indent_first_line(|| self.expr_tail(expr, tail))
            });
            if used_extra_width {
                let lookahead =
                    result.is_ok().then(|| self.out.capture_lookahead(&checkpoint_after_space));
                Next::Wrap(lookahead)
            } else if result.is_err() {
                Next::SameLine
            } else {
                return Ok(());
            }
        };

        self.out.restore_checkpoint(&checkpoint_after_eq);
        match next {
            Next::SameLine => {
                self.out.space()?;
                self.expr(expr)?;
                self.tail(tail)?;
            }
            Next::Wrap(lookahead) => {
                self.indented(|| {
                    self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                    if let Some(lookahead) = lookahead {
                        self.out.restore_lookahead(lookahead);
                    } else {
                        self.expr(expr)?;
                        self.tail(tail)?;
                    }
                    Ok(())
                })?;
            }
        }
        Ok(())
    }
}
