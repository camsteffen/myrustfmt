use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::{EndReserved, Tail};
use crate::source_formatter::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter<'a> {
    pub fn local(&mut self, local: &ast::Local, end: Tail) -> FormatResult {
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

    fn local_init(&mut self, expr: &ast::Expr, end: Tail) -> FormatResult {
        self.out.space()?;
        self.out.token_expect("=")?;
        self.fallback_chain(
            |chain| {
                // single line
                chain.next(|this| {
                    this.with_single_line(|this| {
                        this.out.space()?;
                        this.expr(expr, Tail::None)?;
                        Ok(())
                    })
                });
                // wrap and indent then single line
                chain.next(|this| {
                    this.indented(|this| {
                        this.out.newline_indent()?;
                        this.with_single_line(|this| this.expr(expr, Tail::None))
                    })
                });
                // normal
                chain.next(|this| {
                    this.out.space()?;
                    this.expr(expr, Tail::None)?;
                    Ok(())
                });
                // wrap and indent
                chain.next(|this| {
                    this.indented(|this| {
                        this.out.newline_indent()?;
                        this.expr(expr, Tail::None)?;
                        Ok(())
                    })
                });
            },
            |this| this.tail(end),
        )
    }
}
