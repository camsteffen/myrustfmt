use rustc_ast::ast;

use crate::formatter::{FormatResult, Formatter};

impl<'a> Formatter<'a> {
    pub fn local(&mut self, local: &ast::Local) -> FormatResult {
        let ast::Local {
            pat,
            ty,
            kind,
            attrs,
            span,
            ..
        } = local;
        self.token_space("let", span.lo())?;
        self.pat(pat)?;
        match kind {
            ast::LocalKind::Decl => {
                self.no_space();
                self.token_expect(";")?;
                Ok(())
            }
            ast::LocalKind::Init(expr) => {
                self.space()?;
                self.token_expect("=")?;
                self.fallback_chain("local init")
                    .next("same line and no breaks", |this| {
                        this.with_no_breaks(|this| {
                            this.space()?;
                            this.expr(expr)?;
                            Ok(())
                        })
                    })
                    .next("wrap and indent then one line and no breaks", |this| {
                        this.out.increment_indent();
                        this.newline_indent()?;
                        this.with_no_breaks(|this| {
                            this.expr(expr)?;
                            this.no_space();
                            this.token_expect(";")?;
                            Ok(())
                        })
                    })
                    .next("same line", |this| {
                        this.space()?;
                        this.expr(expr)?;
                        this.no_space();
                        this.token_expect(";")?;
                        Ok(())
                    })
                    .next("wrap and indent", |this| {
                        this.out.increment_indent();
                        this.newline_indent()?;
                        this.expr(expr)?;
                        Ok(())
                    })
                    .result()?;
                Ok(())
            }
            ast::LocalKind::InitElse(_, _) => todo!(),
        }
    }
}
