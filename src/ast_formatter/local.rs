use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::error::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter {
    pub fn local(&self, local: &ast::Local, tail: Tail<'_>) -> FormatResult {
        let ast::Local {
            pat, kind, span, ..
        } = local;
        let pos = span.lo();
        self.out.token_at_space("let", pos)?;
        match kind {
            ast::LocalKind::Decl => self.pat_end(pat, tail),
            ast::LocalKind::Init(init) => {
                self.pat(pat)?;
                self.local_init(init, tail)
            }
            ast::LocalKind::InitElse(init, else_) => {
                self.pat(pat)?;
                self.local_init(init, Tail::NONE)?;
                self.out.space()?;
                self.out.token_expect("else")?;
                self.out.space()?;
                self.block(else_, tail)?;
                Ok(())
            }
        }
    }

    fn local_init(&self, expr: &ast::Expr, end: Tail<'_>) -> FormatResult {
        self.out.space()?;
        self.out.token_expect("=")?;
        // todo do all these cases apply with else clause?
        self.fallback_chain(
            |chain| {
                // single line
                chain.next(|| {
                    self.with_single_line(|| {
                        self.out.space()?;
                        self.expr(expr)?;
                        Ok(())
                    })
                });
                // wrap and indent then single line
                chain.next(|| {
                    self.indented(|| {
                        self.out.newline_indent()?;
                        self.with_single_line(|| self.expr(expr))
                    })
                });
                // normal
                chain.next(|| {
                    self.out.space()?;
                    self.expr(expr)?;
                    Ok(())
                });
                // wrap and indent
                chain.next(|| {
                    self.indented(|| {
                        self.out.newline_indent()?;
                        self.expr(expr)?;
                        Ok(())
                    })
                });
            },
            || self.tail(end),
        )
    }
}
