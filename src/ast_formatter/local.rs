use crate::ast_formatter::AstFormatter;
use crate::source_formatter::FormatResult;
use rustc_ast::ast;
use crate::ast_formatter::last_line::{EndReserved, EndWidth};

impl<'a> AstFormatter<'a> {
    pub fn local(&mut self, local: &ast::Local, end: EndWidth) -> FormatResult<EndReserved> {
        let ast::Local {
            pat,
            kind,
            span,
            ..
        } = local;
        let pos = span.lo();
        self.out.token_at_space("let", pos)?;
        match kind {
            ast::LocalKind::Decl => self.pat_end(pat, end),
            ast::LocalKind::Init(expr) => {
                self.pat(pat)?;
                self.local_init(expr, end)
            },
            ast::LocalKind::InitElse(_, _) => todo!(),
        }
    }

    fn local_init(&mut self, expr: &ast::Expr, end: EndWidth) -> FormatResult<EndReserved> {
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
            .finally(|this| this.reserve_end(end))
            .execute(self)
    }
}
