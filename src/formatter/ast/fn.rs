use rustc_ast::ast;

use crate::formatter::list::ListKind;
use crate::formatter::{FormatResult, Formatter};

impl<'a> Formatter<'a> {
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

    fn fn_sig(&mut self, ast::FnSig { header, decl, span }: &ast::FnSig, item: &ast::Item) {
        self.fn_header(header);
        self.token_expect("fn");
        self.space();
        self.ident(item.ident);
        self.no_space();
        self.fn_decl(decl);
    }

    fn fn_header(
        &mut self,
        ast::FnHeader {
            safety,
            coroutine_kind,
            constness,
            ext,
        }: &ast::FnHeader,
    ) {
        self.safety(safety);
        if let Some(coroutine_kind) = coroutine_kind {
            self.coroutine_kind(coroutine_kind);
        }
        self.constness(constness);
        self.extern_(ext);
    }

    fn fn_decl(&mut self, ast::FnDecl { inputs, output }: &ast::FnDecl) -> FormatResult {
        self.list(ListKind::Parethesis, inputs, |this, param| {
            this.param(param)
        })?;
        self.space();
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
                self.token_expect("->")?;
                self.space()?;
                self.ty(ty);
                self.space()?;
            }
        }
        Ok(())
    }

    fn constness(&mut self, constness: &ast::Const) {
        match *constness {
            ast::Const::Yes(span) => {
                self.token_space("const", span.lo());
            }
            ast::Const::No => {}
        }
    }

    fn extern_(&mut self, ext: &ast::Extern) {
        match *ext {
            ast::Extern::None => {}
            ast::Extern::Implicit(span) => {
                self.token_space("extern", span.lo());
            }
            ast::Extern::Explicit(ref abi, span) => {
                self.token_space("extern", span.lo());
                self.strlit(abi);
                self.space();
            }
        }
    }

    fn safety(&mut self, safety: &ast::Safety) {
        match *safety {
            ast::Safety::Unsafe(span) => {
                self.token_space("unsafe", span.lo());
            }
            ast::Safety::Safe(span) => {
                self.token_space("safe", span.lo());
            }
            ast::Safety::Default => {}
        }
    }

    fn coroutine_kind(&mut self, coroutine_kind: &ast::CoroutineKind) {
        match *coroutine_kind {
            ast::CoroutineKind::Async { span, .. } => {
                self.token_space("async", span.lo());
            }
            ast::CoroutineKind::Gen { span, .. } => {
                self.token_space("gen", span.lo());
            }
            ast::CoroutineKind::AsyncGen { span, .. } => {
                self.token_space("async", span.lo());
                self.token_expect("gen");
                self.space();
            }
        }
    }
}
