use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::ListKind;
use crate::source_formatter::{FormatResult, SourceFormatter};
use rustc_ast::ast;

impl<'a> AstFormatter<'a> {
    pub fn fn_(&mut self, fn_: &ast::Fn, item: &ast::Item) -> FormatResult {
        let ast::Fn {
            defaultness,
            generics,
            sig,
            body,
            ..
        } = fn_;
        self.fn_sig(sig, item);
        if let Some(body) = body {
            self.block(body)?;
        }
        Ok(())
    }

    fn fn_sig(
        &mut self,
        ast::FnSig { header, decl, span }: &ast::FnSig,
        item: &ast::Item,
    ) -> FormatResult {
        self.fn_header(header)?;
        self.out.token_expect("fn")?;
        self.out.space()?;
        self.ident(item.ident)?;
        self.out.no_space();
        self.fn_decl(decl)?;
        Ok(())
    }

    fn fn_header(
        &mut self,
        ast::FnHeader {
            safety,
            coroutine_kind,
            constness,
            ext,
        }: &ast::FnHeader,
    ) -> FormatResult {
        self.safety(safety)?;
        if let Some(coroutine_kind) = coroutine_kind {
            self.coroutine_kind(coroutine_kind)?;
        }
        self.constness(constness)?;
        self.extern_(ext)?;
        Ok(())
    }

    fn fn_decl(&mut self, ast::FnDecl { inputs, output }: &ast::FnDecl) -> FormatResult {
        self.list(ListKind::Parethesis, inputs, |this, param| {
            this.param(param)
        })?;
        self.out.space()?;
        self.fn_ret_ty(output)?;
        Ok(())
    }

    fn param(&mut self, param: &ast::Param) -> FormatResult {
        todo!()
    }

    fn fn_ret_ty(&mut self, output: &ast::FnRetTy) -> FormatResult {
        match output {
            ast::FnRetTy::Default(_) => {}
            ast::FnRetTy::Ty(ty) => {
                self.out.token_expect("->")?;
                self.out.space()?;
                self.ty(ty);
                self.out.space()?;
            }
        }
        Ok(())
    }

    fn constness(&mut self, constness: &ast::Const) -> FormatResult {
        match *constness {
            ast::Const::Yes(span) => {
                let pos = span.lo();
                self.out.token_at_space("const", pos)
            }
            ast::Const::No => Ok(()),
        }
    }

    fn extern_(&mut self, ext: &ast::Extern) -> FormatResult {
        match *ext {
            ast::Extern::None => {}
            ast::Extern::Implicit(span) => {
                let pos = span.lo();
                self.out.token_at_space("extern", pos)?;
            }
            ast::Extern::Explicit(ref abi, span) => {
                let pos = span.lo();
                self.out.token_at_space("extern", pos)?;
                self.strlit(abi);
                self.out.space()?;
            }
        }
        Ok(())
    }

    fn safety(&mut self, safety: &ast::Safety) -> FormatResult {
        match *safety {
            ast::Safety::Unsafe(span) => {
                let pos = span.lo();
                self.out.token_at_space("unsafe", pos)
            }
            ast::Safety::Safe(span) => {
                let pos = span.lo();
                self.out.token_at_space("safe", pos)
            }
            ast::Safety::Default => Ok(()),
        }
    }

    fn coroutine_kind(&mut self, coroutine_kind: &ast::CoroutineKind) -> FormatResult {
        match *coroutine_kind {
            ast::CoroutineKind::Async { span, .. } => {
                let pos = span.lo();
                self.out.token_at_space("async", pos)?;
                Ok(())
            }
            ast::CoroutineKind::Gen { span, .. } => {
                let pos = span.lo();
                self.out.token_at_space("gen", pos)?;
                Ok(())
            }
            ast::CoroutineKind::AsyncGen { span, .. } => {
                let pos = span.lo();
                self.out.token_at_space("async", pos)?;
                self.out.token_expect("gen")?;
                self.out.space()?;
                Ok(())
            }
        }
    }
}
