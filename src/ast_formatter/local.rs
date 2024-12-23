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
            ast::LocalKind::Decl => self.pat_tail(pat, tail),
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
                self.block(else_)?;
                self.tail(tail)?;
                Ok(())
            }
        }
    }

    fn local_init(&self, expr: &ast::Expr, end: Tail<'_>) -> FormatResult {
        self.out.space()?;
        self.out.token_expect("=")?;
        // todo do all these cases apply with else clause?
        // single line
        // self.fallback(|| {
        //     self.with_single_line(|| {
        //         self.out.space()?;
        //         self.expr(expr)?;
        //         self.tail(end)?;
        //         Ok(())
        //     })
        // })
        // wrap and indent then single line
        // .next(|| {
        //     self.indented(|| {
        //         self.out.newline_indent()?;
        //         self.with_single_line(|| self.with_no_overflow(|| self.expr(expr)))?;
        //         self.tail(end)?;
        //         Ok(())
        //     })
        // })
        // normal
        self.fallback(|| {
        // .next(|| {
            self.out.space()?;
            self.expr(expr)?;
            self.tail(end)?;
            Ok(())
        })
        // wrap and indent
        .next(|| {
            self.indented(|| {
                self.out.newline_indent()?;
                self.expr(expr)?;
                self.tail(end)?;
                Ok(())
            })
        })
        .result()
    }
}
