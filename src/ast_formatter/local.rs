use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::tail::Tail;
use crate::ast_utils::expr_only_block;
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;

impl AstFormatter {
    pub fn local(&self, local: &ast::Local) -> FormatResult {
        self.with_attrs(&local.attrs, local.span, || self.local_after_attrs(local))
    }

    fn local_after_attrs(&self, local: &ast::Local) -> FormatResult {
        let ast::Local { pat, kind, .. } = local;
        let start = self.out.last_line_len();
        self.out.token_space("let")?;
        match kind {
            ast::LocalKind::Decl => self.pat_tail(pat, &Tail::token(";")),
            ast::LocalKind::Init(init) => {
                self.pat(pat)?;
                self.local_init(init, &Tail::token(";"))
            }
            ast::LocalKind::InitElse(init, else_) => {
                self.pat(pat)?;
                self.local_init(init, Tail::none())?;
                let else_block = || {
                    self.block_after_open_brace(else_)?;
                    self.out.token(";")?;
                    Ok(())
                };
                self.fallback(|| {
                    self.out.space_token_space("else")?;
                    self.out.token("{")?;
                    match expr_only_block(else_) {
                        None => else_block()?,
                        Some(else_expr) => self
                            .fallback(|| {
                                self.with_width_limit_from_start(
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
        }
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
