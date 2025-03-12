use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::tail::Tail;
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
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
        let else_separate_lines = || {
            self.block_expr_vertical_after_open_brace(else_)?;
            self.out.token(";")?;
            Ok(())
        };
        let same_line_else = || -> FormatResult {
            self.out.space_token_space("else")?;
            self.out.token("{")?;
            let expr_only_else = if is_single_line_init {
                self.try_into_expr_only_block(else_)
            } else {
                None
            };
            let Some(expr_only_else) = expr_only_else else {
                return else_separate_lines();
            };
            self.backtrack()
                .next(|| {
                    self.with_width_limit_from_start(
                        start,
                        RUSTFMT_CONFIG_DEFAULTS.single_line_let_else_max_width,
                        || {
                            self.with_single_line(|| {
                                self.expr_only_block_after_open_brace(expr_only_else)?;
                                self.out.token(";")?;
                                Ok(())
                            })
                        },
                    )
                })
                .otherwise(else_separate_lines)?;
            Ok(())
        };
        let next_line_else = || -> FormatResult {
            self.newline_break_indent()?;
            self.out.token_space("else")?;
            self.out.token("{")?;
            else_separate_lines()?;
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
        // let checkpoint = self.out.checkpoint();
        // self.with_single_line(|| {
        //     self.out.constraints().with_max_width(Some(self.out.constraints().max))
        // })
        // todo do all these cases apply with else clause?
        // single line
        self.backtrack()
            .next(|| {
                self.with_single_line(|| {
                    self.out.space()?;
                    self.expr(expr)?;
                    self.tail(tail)?;
                    Ok(())
                })
            })
            // todo use lookahead to avoid re-formatting
            // wrap and indent then single line
            .next(|| {
                self.indented(|| {
                    self.newline_break_indent()?;
                    self.with_single_line(|| self.expr(expr))?;
                    self.tail(tail)?;
                    Ok(())
                })
            })
            // normal
            .next(|| {
                self.out.space()?;
                self.expr(expr)?;
                self.tail(tail)?;
                Ok(())
            })
            // wrap and indent
            .otherwise(|| {
                self.indented(|| {
                    self.newline_break_indent()?;
                    self.expr(expr)?;
                    self.tail(tail)?;
                    Ok(())
                })
            })
    }
}
