use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{AngleBracketedArgsConfig, ListConfig, ParamListConfig};
use crate::source_formatter::FormatResult;
use rustc_ast::ast;
use crate::ast_formatter::last_line::{EndReserved, Tail};

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
            self.block(body, Tail::None)?;
        }
        Ok(())
    }

    pub fn closure(&mut self, closure: &ast::Closure, end: Tail) -> FormatResult {
        match closure.binder {
            ast::ClosureBinder::NotPresent => {}
            ast::ClosureBinder::For { span, ref generic_params } => todo!(),
        }
        match closure.capture_clause {
            ast::CaptureBy::Ref => {}
            ast::CaptureBy::Value { move_kw } => {
                self.out.token_at_space("move", move_kw.lo())?
            },
        }
        self.constness(closure.constness)?;
        if let Some(coroutine_kind) = &closure.coroutine_kind {
            self.coroutine_kind(coroutine_kind)?;
        }
        self.fn_decl(&closure.fn_decl, ClosureParamListConfig)?;
        self.out.space()?;
        self.expr(&closure.body, end)
    }

    pub fn parenthesized_args(&mut self, parenthesized_args: &ast::ParenthesizedArgs) -> FormatResult {
        self.list(&parenthesized_args.inputs, |this, ty| this.ty(ty), ParamListConfig, Tail::None)?;
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
        if !generics.params.is_empty() {
            self.list(&generics.params, Self::generic_param, AngleBracketedArgsConfig, Tail::None)?;
        }
        self.fn_decl(decl, ParamListConfig)?;
        Ok(())
    }

    fn generic_param(&mut self, param: &ast::GenericParam) -> FormatResult {
        self.ident(param.ident)?;
        self.generic_bounds(&param.bounds)?;
        match param.kind {
            ast::GenericParamKind::Const {ref ty,kw_span, ref default} => {
                self.out.token_at_space("const", kw_span.lo())?;
                if let Some(default) = default {
                    todo!()
                }
                self.ty(ty)?;
            }
            ast::GenericParamKind::Lifetime => {}
            ast::GenericParamKind::Type {ref default} => {
                if let Some(default) = default {
                    todo!()
                }
            }
        }
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

    fn fn_decl(&mut self, ast::FnDecl { inputs, output }: &ast::FnDecl, input_list_config: impl ListConfig) -> FormatResult {
        self.list(inputs, |this, param| this.param(param), input_list_config,Tail::None)?;
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