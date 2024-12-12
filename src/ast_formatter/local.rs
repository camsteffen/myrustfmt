use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::{EndReserved, EndWidth};
use crate::source_formatter::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter<'a> {
    pub fn local(&mut self, local: &ast::Local, end: EndWidth) -> FormatResult<EndReserved> {
        let ast::Local {
            pat, kind, span, ..
        } = local;
        let pos = span.lo();
        self.out.token_at_space("let", pos)?;
        match kind {
            ast::LocalKind::Decl => self.pat_end(pat, end),
            ast::LocalKind::Init(expr) => {
                self.pat(pat)?;
                self.local_init(expr, end)
            }
            ast::LocalKind::InitElse(_, _) => todo!(),
        }
    }

    fn local_init(&mut self, expr: &ast::Expr, end: EndWidth) -> FormatResult<EndReserved> {
        self.out.space()?;
        self.out.token_expect("=")?;
        self.fallback_chain(
            |chain| {
                // single line
                chain.next(|this| {
                    this.with_single_line(|this| {
                        this.out.space()?;
                        this.expr(expr)?;
                        Ok(())
                    })
                });
                // wrap and indent then single line
                chain.next(|this| {
                    this.with_indent(|this| {
                        this.out.newline_indent()?;
                        this.with_single_line(|this| this.expr(expr))
                    })
                });
                // normal
                chain.next(|this| {
                    this.out.space()?;
                    this.expr(expr)?;
                    Ok(())
                });
                // wrap and indent
                chain.next(|this| {
                    this.with_indent(|this| {
                        this.out.newline_indent()?;
                        this.expr(expr)?;
                        Ok(())
                    })
                });
            },
            |this| this.reserve_end(end),
        )
    }
}
