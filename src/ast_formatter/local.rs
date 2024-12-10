use crate::ast_formatter::AstFormatter;
use crate::source_formatter::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter<'a> {
    pub fn local(&mut self, local: &ast::Local, finally: impl Fn(&mut Self) -> FormatResult) -> FormatResult {
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
            ast::LocalKind::Decl => finally(self),
            ast::LocalKind::Init(expr) => self.local_init(expr, finally),
            ast::LocalKind::InitElse(_, _) => todo!(),
        }
    }

    fn local_init(&mut self, expr: &ast::Expr, finally: impl Fn(&mut Self) -> FormatResult) -> FormatResult {
        self.out.space()?;
        self.out.token_expect("=")?;
        self.fallback_chain("local init")
            .next("single line", |this| {
                this.with_single_line(|this| {
                    this.out.space()?;
                    this.expr(expr)?;
                    Ok(())
                })
            })
            .next("wrap and indent then single line", |this| {
                this.with_indent(|this| {
                    this.out.newline_indent()?;
                    this.with_single_line(|this| this.expr(expr))
                })
            })
            .next("normal", |this| {
                this.out.space()?;
                this.expr(expr)?;
                Ok(())
            })
            .next("wrap and indent", |this| {
                this.with_indent(|this| {
                    this.out.newline_indent()?;
                    this.expr(expr)?;
                    Ok(())
                })
            })
            .finally(finally)
            .execute(self)?;
        Ok(())
    }
}
