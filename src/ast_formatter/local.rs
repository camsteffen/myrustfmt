use crate::ast_formatter::AstFormatter;
use crate::source_formatter::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter<'a> {
    pub fn local(&mut self, local: &ast::Local) -> FormatResult {
        let ast::Local {
            pat,
            kind,
            span,
            ..
        } = local;
        let pos = span.lo();
        self.out.token_at_space("let", pos)?;
        self.pat(pat)?;
        match kind {
            ast::LocalKind::Decl => Ok(()),
            ast::LocalKind::Init(expr) => self.local_init(expr),
            ast::LocalKind::InitElse(_, _) => todo!(),
        }
    }

    fn local_init(&mut self, expr: &ast::Expr) -> FormatResult {
        self.fallback_chain("local init")
            .next("single line", |this| {
                this.with_single_line(|this| {
                    this.out.space()?;
                    this.out.token_expect("=")?;
                    this.out.space()?;
                    this.expr(expr)?;
                    Ok(())
                })
            })
            .next("wrap and indent then single line", |this| {
                this.with_indent(|this| {
                    this.with_leading_lines(|this| {
                        this.out.space()?;
                        this.out.token_expect("=")?;
                        Ok(())
                    })?;
                    this.out.newline_indent()?;
                    this.with_single_line(|this| this.expr(expr))
                })
            })
            .next("normal", |this| {
                this.out.space()?;
                this.out.token_expect("=")?;
                this.out.space()?;
                this.expr(expr)?;
                Ok(())
            })
            .next("wrap and indent", |this| {
                this.with_leading_lines(|this| {
                    this.out.space()?;
                    this.out.token_expect("=")?;
                    Ok(())
                })?;
                this.with_indent(|this| {
                    this.out.newline_indent()?;
                    this.expr(expr)?;
                    Ok(())
                })
            })
            .execute(self)?;
        Ok(())
    }
}
