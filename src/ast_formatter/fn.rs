use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::{ListConfig, list_overflow_no, param_list_config};
use crate::source_formatter::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter<'a> {
    pub fn fn_(&mut self, fn_: &ast::Fn, item: &ast::Item) -> FormatResult {
        let ast::Fn {
            generics,
            sig,
            body,
            ..
        } = fn_;
        self.fn_sig(sig, generics, item)?;
        if let Some(body) = body {
            self.out.space()?;
            self.block(body, Tail::NONE)?;
        }
        Ok(())
    }

    pub fn closure(&mut self, closure: &ast::Closure, end: Tail) -> FormatResult {
        match closure.binder {
            ast::ClosureBinder::NotPresent => {}
            ast::ClosureBinder::For {
                span,
                ref generic_params,
            } => todo!(),
        }
        match closure.capture_clause {
            ast::CaptureBy::Ref => {}
            ast::CaptureBy::Value { move_kw } => self.out.token_at_space("move", move_kw.lo())?,
        }
        self.constness(closure.constness)?;
        if let Some(coroutine_kind) = &closure.coroutine_kind {
            self.coroutine_kind(coroutine_kind)?;
        }
        self.fn_decl(&closure.fn_decl, ClosureParamListConfig)?;
        self.out.space()?;
        self.expr(&closure.body, end)
    }

    pub fn parenthesized_args(
        &mut self,
        parenthesized_args: &ast::ParenthesizedArgs,
    ) -> FormatResult {
        self.list(
            &parenthesized_args.inputs,
            |this, ty| this.ty(ty),
            param_list_config(None),
            list_overflow_no(),
            Tail::NONE,
        )?;
        self.fn_ret_ty(&parenthesized_args.output)?;
        Ok(())
    }

    fn fn_sig(
        &mut self,
        ast::FnSig { header, decl, span }: &ast::FnSig,
        generics: &ast::Generics,
        item: &ast::Item,
    ) -> FormatResult {
        self.fn_header(header)?;
        self.out.token_expect("fn")?;
        self.out.space()?;
        self.ident(item.ident)?;
        self.generics(generics)?;
        self.fn_decl(decl, param_list_config(None))?;
        Ok(())
    }

    fn fn_header(
        &mut self,
        &ast::FnHeader {
            ref safety,
            ref coroutine_kind,
            constness,
            ref ext,
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

    fn fn_decl(
        &mut self,
        ast::FnDecl { inputs, output }: &ast::FnDecl,
        input_list_config: impl ListConfig,
    ) -> FormatResult {
        self.list(
            inputs,
            |this, param| this.param(param),
            input_list_config,
            list_overflow_no(),
            Tail::NONE,
        )?;
        self.fn_ret_ty(output)?;
        Ok(())
    }

    fn param(&mut self, param: &ast::Param) -> FormatResult {
        tracing::info!("{:?}", param);
        self.attrs(&param.attrs)?;
        self.pat(&param.pat)?;
        if !matches!(param.ty.kind, ast::TyKind::Infer) {
            self.out.token_expect(":")?;
            self.out.space()?;
            self.ty(&param.ty)?;
        }
        Ok(())
    }

    fn fn_ret_ty(&mut self, output: &ast::FnRetTy) -> FormatResult {
        match output {
            ast::FnRetTy::Default(_) => {}
            ast::FnRetTy::Ty(ty) => {
                self.out.space()?;
                self.out.token_expect("->")?;
                self.out.space()?;
                self.ty(ty)?;
            }
        }
        Ok(())
    }

    fn constness(&mut self, constness: ast::Const) -> FormatResult {
        match constness {
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

struct ClosureParamListConfig;

impl ListConfig for ClosureParamListConfig {
    const START_BRACE: &'static str = "|";
    const END_BRACE: &'static str = "|";
    const PAD_CONTENTS: bool = false;
}
