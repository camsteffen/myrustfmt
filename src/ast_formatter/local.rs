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
            // todo tail?
            // self.out.token(";")?;
            return Ok(());
        };
        self.pat(pat)?;
        if let Some(ty) = ty {
            // todo tail?
            self.out.token_space(":")?;
            self.ty(ty)?;
        }
        // "else else else" lol
        let Some(else_) = else_ else {
            self.local_init(init, &Tail::token(";"))?;
            return Ok(());
        };
        self.local_init(init, Tail::none())?;
        let else_block = || {
            self.block_after_open_brace(else_)?;
            self.out.token(";")?;
            Ok(())
        };
        self.fallback(|| {
            self.out.space_token_space("else")?;
            self.out.token("{")?;
            match self.expr_only_block(else_) {
                None => else_block()?,
                Some(else_expr) => self
                    .fallback(|| {
                        self.with_width_limit_from_start(
                            // todo verify still on the same line
                            start,
                            RUSTFMT_CONFIG_DEFAULTS.single_line_let_else_max_width,
                            || {
                                self.with_single_line(|| {
                                    self.out.space()?;
                                    self.expr(else_expr)?;
                                    self.out.space_token("}")?;
                                    self.out.token(";")?;
                                    Ok(())
                                })
                            },
                        )
                    })
                    .otherwise(else_block)?,
            }
            Ok(())
        })
        .otherwise(|| {
            self.out.newline_within_indent()?;
            self.out.token_space("else")?;
            self.out.token("{")?;
            else_block()?;
            Ok(())
        })?;
        Ok(())
    }

    fn local_init(&self, expr: &ast::Expr, end: &Tail) -> FormatResult {
        self.out.space_token("=")?;
        // todo do all these cases apply with else clause?
        // single line
        self.fallback(|| {
            self.with_single_line(|| {
                self.out.space()?;
                self.expr(expr)?;
                self.tail(end)?;
                Ok(())
            })
        })
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
