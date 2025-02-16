use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::tail::Tail;
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
                self.pat_tail(pat, &Tail::token(";"))?;
                return Ok(());
            };
            self.pat(pat)?;
            // todo tail?
            self.out.token_space(":")?;
            self.ty_tail(ty, &Tail::token(";"))?;
            return Ok(());
        };
        self.pat(pat)?;
        if let Some(ty) = ty {
            // todo tail?
            self.out.token_space(":")?;
            self.ty(ty)?;
        }
        // else else else lol
        let Some(else_) = else_ else {
            self.local_init(init, &Tail::token(";"))?;
            return Ok(());
        };
        self.local_init(init, Tail::none())?;
        let is_single_line_init = self.out.line() == first_line;
        let else_separate_lines = || {
            self.block_separate_lines_after_open_brace(else_)?;
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
            self.out.newline_within_indent()?;
            self.out.token_space("else")?;
            self.out.token("{")?;
            else_separate_lines()?;
            Ok(())
        };
        if is_single_line_init || self.out.last_line_is_closers() {
            self.backtrack()
                .next(same_line_else)
                .otherwise(next_line_else)?;
        } else {
            next_line_else()?;
        }
        Ok(())
    }

    fn local_init(&self, expr: &ast::Expr, end: &Tail) -> FormatResult {
        self.out.space_token("=")?;
        // todo do all these cases apply with else clause?
        // single line
        self.backtrack()
            .next_single_line(|| {
                self.out.space()?;
                self.expr(expr)?;
                self.tail(end)?;
                Ok(())
            })
            // todo use lookahead to avoid re-formatting
            // wrap and indent then single line
            .next(|| {
                self.indented(|| {
                    self.out.newline_within_indent()?;
                    self.with_single_line(|| self.expr(expr))?;
                    self.tail(end)?;
                    Ok(())
                })
            })
            // normal
            .next(|| {
                self.out.space()?;
                self.expr(expr)?;
                self.tail(end)?;
                Ok(())
            })
            // wrap and indent
            .otherwise(|| {
                self.indented(|| {
                    self.out.newline_within_indent()?;
                    self.expr(expr)?;
                    self.tail(end)?;
                    Ok(())
                })
            })
    }
}
