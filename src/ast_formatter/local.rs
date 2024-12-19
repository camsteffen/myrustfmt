use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::{Tail};
use crate::source_formatter::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter {
    pub fn local(&self, local: &ast::Local, end: Tail<'_>) -> FormatResult {
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

    fn local_init(&self, expr: &ast::Expr, end: Tail<'_>) -> FormatResult {
        self.out.space()?;
        self.out.token_expect("=")?;
        self.fallback_chain(
            |chain| {
                // single line
                chain.next(|| {
                    self.with_single_line(|| {
                        self.out.space()?;
                        self.expr(expr, Tail::NONE)?;
                        Ok(())
                    })
                });
                // wrap and indent then single line
                chain.next(|| {
                    self.indented(|| {
                        self.out.newline_indent()?;
                        self.with_single_line(|| self.expr(expr, Tail::NONE))
                    })
                });
                // normal
                chain.next(|| {
                    self.out.space()?;
                    self.expr(expr, Tail::NONE)?;
                    Ok(())
                });
                // wrap and indent
                chain.next(|| {
                    self.indented(|| {
                        self.out.newline_indent()?;
                        self.expr(expr, Tail::NONE)?;
                        Ok(())
                    })
                });
            },
            || self.tail(end),
        )
    }
}
