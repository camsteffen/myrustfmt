use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::{ListConfig, list, param_list_config};
use crate::source_formatter::FormatResult;

use rustc_ast::ast;

impl<'a> AstFormatter {
    pub fn fn_<K>(&self, fn_: &ast::Fn, item: &ast::Item<K>) -> FormatResult {
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

    pub fn closure(
        &self,
        closure: &ast::Closure,
        is_overflow: bool,
        end: Tail<'_>,
    ) -> FormatResult {
        match closure.binder {
            ast::ClosureBinder::NotPresent => {}
            ast::ClosureBinder::For {
                span: _,
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

        if is_overflow {
            self.with_not_single_line(|| self.closure_body(&closure.body, end))?;
        } else {
            self.closure_body(&closure.body, end)?;
        }
        Ok(())
    }

    fn closure_body(&self, body: &ast::Expr, tail: Tail<'_>) -> FormatResult {
        let mut inner_expr = None;
        if let ast::ExprKind::Block(block, None) = &body.kind {
            if matches!(block.rules, ast::BlockCheckMode::Default) {
                if let [stmt] = &block.stmts[..] {
                    if let ast::StmtKind::Expr(expr) = &stmt.kind {
                        if expr.attrs.is_empty() {
                            inner_expr = Some(expr);
                        }
                    }
                }
            }
        }
        if let Some(expr) = inner_expr {
            self.out.skip_token("{");
            self.closure_body(expr, tail)?;
            self.out.skip_token("}");
            return Ok(());
        }
        fn allow_multi_line(expr: &ast::Expr) -> bool {
            match expr.kind {
                ast::ExprKind::Match(..)
                | ast::ExprKind::Gen(..)
                | ast::ExprKind::Block(..)
                | ast::ExprKind::TryBlock(..)
                | ast::ExprKind::Loop(..)
                | ast::ExprKind::Struct(..) => true,

                ast::ExprKind::AddrOf(_, _, ref expr)
                | ast::ExprKind::Try(ref expr)
                | ast::ExprKind::Unary(_, ref expr)
                | ast::ExprKind::Cast(ref expr, _) => allow_multi_line(expr),

                _ => false,
            }
        }
        if allow_multi_line(body) {
            self.expr(body, tail)
        } else {
            self.fallback_chain(
                |chain| {
                    chain.next(|| self.with_single_line(|| self.expr(body, tail)));
                    chain.next(|| {
                        self.out.token_missing("{")?;
                        self.indented(|| {
                            self.out.newline_indent()?;
                            self.expr(body, tail)?;
                            Ok(())
                        })?;
                        self.out.newline_indent()?;
                        self.out.token_missing("}")?;
                        Ok(())
                    });
                },
                || Ok(()),
            )
        }
    }

    pub fn parenthesized_args(
        &self,
        parenthesized_args: &ast::ParenthesizedArgs,
    ) -> FormatResult {
        list(
            &parenthesized_args.inputs,
            |ty| self.ty(ty),
            param_list_config(None),
        )
        .format(self)?;
        self.fn_ret_ty(&parenthesized_args.output)?;
        Ok(())
    }

    fn fn_sig<K>(
        &self,
        ast::FnSig { header, decl, span }: &ast::FnSig,
        generics: &ast::Generics,
        item: &ast::Item<K>,
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
        &self,
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
        &self,
        ast::FnDecl { inputs, output }: &ast::FnDecl,
        input_list_config: impl ListConfig,
    ) -> FormatResult {
        list(inputs, |param| self.param(param), input_list_config)
            .tail(Tail::new(&|| self.fn_ret_ty(output)))
            .format(self)?;
        Ok(())
    }

    fn param(&self, param: &ast::Param) -> FormatResult {
        tracing::info!("{:?}", param);
        self.attrs(&param.attrs)?;
        if param.is_self() {
            self.ty(&param.ty)?;
            return Ok(());
        }
        self.pat(&param.pat)?;
        if !matches!(param.ty.kind, ast::TyKind::Infer) {
            self.out.token_expect(":")?;
            self.out.space()?;
            self.ty(&param.ty)?;
        }
        Ok(())
    }

    fn fn_ret_ty(&self, output: &ast::FnRetTy) -> FormatResult {
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

    fn constness(&self, constness: ast::Const) -> FormatResult {
        match constness {
            ast::Const::Yes(span) => {
                let pos = span.lo();
                self.out.token_at_space("const", pos)
            }
            ast::Const::No => Ok(()),
        }
    }

    fn extern_(&self, ext: &ast::Extern) -> FormatResult {
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

    fn coroutine_kind(&self, coroutine_kind: &ast::CoroutineKind) -> FormatResult {
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
