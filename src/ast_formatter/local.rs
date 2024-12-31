use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter {
    pub fn local(&self, local: &ast::Local, tail: &Tail) -> FormatResult {
        self.with_attrs_tail(&local.attrs, local.span, tail, || {
            self.local_after_attrs(local, tail)
        })
    }

    fn local_after_attrs(&self, local: &ast::Local, tail: &Tail) -> FormatResult {
        let ast::Local { pat, kind, .. } = local;
        self.out.token_space("let")?;
        match kind {
            ast::LocalKind::Decl => self.pat_tail(pat, tail),
            ast::LocalKind::Init(init) => {
                self.pat(pat)?;
                self.local_init(init, tail)
            }
            ast::LocalKind::InitElse(init, else_) => {
                self.pat(pat)?;
                self.local_init(init, Tail::NONE)?;
                self.fallback(|| {
                    self.out.space_token_space("else")?;
                    self.out.token("{")?;
                    Ok(())
                })
                .next(|| {
                    self.out.newline_indent()?;
                    self.out.token_space("else")?;
                    self.out.token("{")?;
                    Ok(())
                })
                .result()?;
                self.block_after_open_brace(else_)?;
                self.tail(tail)?;
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
                self.out.newline_indent()?;
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
