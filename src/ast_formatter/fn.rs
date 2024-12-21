use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::{Braces,  list};
use crate::error::FormatResult;

use rustc_ast::ast;
use rustc_span::symbol::kw;
use crate::ast_formatter::list::config::{DefaultListConfig, ListConfig, ParamListConfig};

impl<'a> AstFormatter {
    pub fn fn_<K>(&self, fn_: &ast::Fn, item: &ast::Item<K>) -> FormatResult {
        let ast::Fn {
            generics,
            sig,
            body,
            ..
        } = fn_;
        self.fn_header(&sig.header)?;
        self.out.token_expect("fn")?;
        self.out.space()?;
        self.ident(item.ident)?;
        self.generic_params(&generics.params)?;
        let (decl_tail, opened_block) = if generics.where_clause.is_empty() && body.is_some() {
            (Tail::OPEN_BLOCK, true)
        } else {
            (Tail::NONE, false)
        };
        self.fn_decl(
            &sig.decl,
            Braces::PARENS,
            &ParamListConfig {
                single_line_max_contents_width: None,
            },
            decl_tail,
        )?;
        self.where_clause(&generics.where_clause)?;
        if let Some(body) = body {
            if opened_block {
                self.block_after_open_brace(body)?;
            } else {
                self.block(body)?;
            }
        } else {
            self.out.token_expect(";")?;
        }
        Ok(())
    }

    pub fn closure(
        &self,
        closure: &ast::Closure,
        is_overflow: bool,
        end: Tail<'_>,
    ) -> FormatResult {
        let ast::Closure {
            ref binder,
            capture_clause,
            constness,
            ref coroutine_kind,
            ref fn_decl,
            ref body,
            ..
        } = *closure;
        match *binder {
            ast::ClosureBinder::NotPresent => {}
            ast::ClosureBinder::For {
                span: _,
                generic_params: _,
            } => todo!(),
        }
        match capture_clause {
            ast::CaptureBy::Ref => {}
            ast::CaptureBy::Value { move_kw } => self.out.token_at_space("move", move_kw.lo())?,
        }
        self.constness(constness)?;
        if let Some(coroutine_kind) = coroutine_kind {
            self.coroutine_kind(coroutine_kind)?;
        }
        self.fn_decl(fn_decl, Braces::PIPE, &DefaultListConfig, Tail::NONE)?;
        self.out.space()?;

        if is_overflow {
            self.with_not_single_line(|| self.closure_body(body, end))?;
        } else {
            self.closure_body(body, end)?;
        }
        Ok(())
    }

    fn closure_body(&self, body: &ast::Expr, tail: Tail<'_>) -> FormatResult {
        fn is_block_like(expr: &ast::Expr) -> bool {
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
                | ast::ExprKind::Cast(ref expr, _) => is_block_like(expr),

                _ => false,
            }
        }
        self.skip_single_expr_blocks(body, |body| {
            if is_block_like(body) {
                self.expr_tail(body, tail)
            } else {
                self.fallback(|| {
                    self.with_no_overflow(|| self.with_single_line(|| self.expr_tail(body, tail)))
                })
                .next(|| {
                    self.add_block(|| self.expr(body))?;
                    self.tail(tail)?;
                    Ok(())
                })
                .result()
            }
        })
    }

    pub fn bare_fn_ty(&self, bare_fn_ty: &ast::BareFnTy) -> FormatResult {
        // self.safety(&bare_fn_ty.safety)?;
        // self.extern_(&bare_fn_ty.ext)?;
        // self.generic_params(&bare_fn_ty.generic_params)?;
        self.out.token_expect("fn")?;
        self.fn_decl(&bare_fn_ty.decl, Braces::PARENS, &DefaultListConfig, Tail::NONE)?;
        Ok(())
    }

    pub fn parenthesized_args(&self, parenthesized_args: &ast::ParenthesizedArgs) -> FormatResult {
        list(Braces::PARENS, &parenthesized_args.inputs, |ty| self.ty(ty))
            .config(&ParamListConfig {
                single_line_max_contents_width: None,
            })
            .format(self)?;
        self.fn_ret_ty(&parenthesized_args.output)?;
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

    fn fn_decl<C: ListConfig>(
        &self,
        fn_decl: &ast::FnDecl,
        braces: &'static Braces,
        input_list_config: &C,
        tail: Tail<'_>,
    ) -> FormatResult {
        // args and return type all on one line
        self.fallback(|| {
            list(braces, &fn_decl.inputs, |param| self.param(param))
                .config(input_list_config)
                .format_single_line(self)?;
            self.with_single_line(|| self.fn_ret_ty(&fn_decl.output))?;
            self.tail(tail)?;
            Ok(())
        })
        // args on separate lines
        .next(|| {
            list(braces, &fn_decl.inputs, |param| self.param(param))
                .config(input_list_config)
                .format_separate_lines(self)?;
            self.fn_ret_ty(&fn_decl.output)?;
            self.tail(tail)?;
            Ok(())
        })
        .result()
    }

    fn param(&self, param: &ast::Param) -> FormatResult {
        self.attrs(&param.attrs)?;
        if let ast::PatKind::Ident(_, ident, _) = param.pat.kind {
            if matches!(ident.name, kw::Empty | kw::SelfLower) {
                // type only; e.g. `&self` or `fn(String)`
                self.ty(&param.ty)?;
                return Ok(())
            }
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
                self.strlit(abi)?;
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
