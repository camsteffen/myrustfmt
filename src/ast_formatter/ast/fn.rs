use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{Braces, ListItemContext};
use crate::ast_formatter::tail::Tail;
use crate::error::FormatResult;

use crate::ast_formatter::list::options::{ListOptions, ListShape};
use crate::constraints::VStruct;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::BindingMode;
use rustc_ast::ast;
use rustc_span::symbol::kw;

impl AstFormatter {
    pub fn fn_<K>(&self, fn_: &ast::Fn, item: &ast::Item<K>) -> FormatResult {
        let ast::Fn {
            generics,
            sig,
            body,
            ..
        } = fn_;
        self.fn_header(&sig.header)?;
        self.token_ident_generic_params("fn", item.ident, generics)?;
        let is_block_after_decl = generics.where_clause.is_empty() && body.is_some();
        self.fn_decl(
            &sig.decl,
            Braces::Parens,
            &self.tail_fn(|af| {
                if is_block_after_decl {
                    af.out.space_token("{")?;
                }
                Ok(())
            }),
        )?;
        self.where_clause(&generics.where_clause, body.is_some())?;
        if let Some(body) = body {
            self.block_expr(is_block_after_decl, body)?;
        } else {
            self.out.token(";")?;
        }
        Ok(())
    }

    pub fn closure(&self, closure: &ast::Closure, tail: Tail) -> FormatResult {
        self.has_vstruct(VStruct::Closure, || {
            let first_line = self.out.line();
            match closure.binder {
                ast::ClosureBinder::NotPresent => {}
                ast::ClosureBinder::For { .. } => todo!(),
            }
            match closure.capture_clause {
                ast::CaptureBy::Ref => {}
                ast::CaptureBy::Value { .. } => self.out.token_space("move")?,
            }
            self.constness(closure.constness)?;
            if let Some(coroutine_kind) = &closure.coroutine_kind {
                self.coroutine_kind(coroutine_kind)?;
            }
            self.fn_decl(
                &closure.fn_decl,
                Braces::Pipe,
                &self.tail_fn(|af| {
                    af.out.space()?;
                    let has_return_type = match closure.fn_decl.output {
                        ast::FnRetTy::Default(_) => false,
                        ast::FnRetTy::Ty(_) => true,
                    };
                    let multi_line_header = self.out.line() == first_line;
                    af.closure_body(&closure.body, has_return_type, multi_line_header, tail)?;
                    Ok(())
                }),
            )?;
            Ok(())
        })
    }

    fn closure_body(
        &self,
        body: &ast::Expr,
        has_return_type: bool,
        single_line_header: bool,
        tail: Tail,
    ) -> FormatResult {
        if has_return_type {
            return self.expr_tail(body, tail);
        }
        if !single_line_header {
            self.expr_force_plain_block(body)?;
            self.tail(tail)?;
            return Ok(());
        }
        self.skip_single_expr_blocks_tail(body, tail, |body, tail| {
            self.backtrack()
                // todo recover width?
                .next(|| {
                    self.disallow_vstructs(
                        VStruct::Closure
                            | VStruct::ControlFlow
                            | VStruct::HangingIndent
                            | VStruct::List,
                        || self.expr_tail(body, tail),
                    )
                })
                .next(|| {
                    self.expr_add_block(body)?;
                    self.tail(tail)?;
                    Ok(())
                })
                .result()
        })
    }

    pub fn bare_fn_ty(&self, bare_fn_ty: &ast::BareFnTy) -> FormatResult {
        // todo
        // self.safety(&bare_fn_ty.safety)?;
        // self.extern_(&bare_fn_ty.ext)?;
        // self.generic_params(&bare_fn_ty.generic_params)?;
        self.out.token("fn")?;
        self.fn_decl(&bare_fn_ty.decl, Braces::Parens, &None)?;
        Ok(())
    }

    pub fn parenthesized_args(
        &self,
        parenthesized_args: &ast::ParenthesizedArgs,
        tail: Tail,
    ) -> FormatResult {
        let (list_tail, final_tail) = match parenthesized_args.output {
            ast::FnRetTy::Default(_) => (tail, &None),
            ast::FnRetTy::Ty(_) => (&None, tail),
        };
        self.list(
            Braces::Parens,
            &parenthesized_args.inputs,
            |af, ty, tail, _lcx| af.ty_tail(ty, tail),
            ListOptions::new().tail(list_tail),
        )?;
        self.fn_ret_ty(&parenthesized_args.output)?;
        // todo pass tail to ret ty?
        self.tail(final_tail)?;
        Ok(())
    }

    fn fn_header(
        &self,
        &ast::FnHeader {
            safety,
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

    fn fn_decl(&self, fn_decl: &ast::FnDecl, braces: Braces, tail: Tail) -> FormatResult {
        let params = |shape| {
            self.list(
                braces,
                &fn_decl.inputs,
                Self::param,
                ListOptions::new().shape(shape),
            )
        };
        // args and return type all on one line
        self.backtrack()
            .next(|| {
                self.with_single_line(|| {
                    params(ListShape::Horizontal)?;
                    self.fn_ret_ty(&fn_decl.output)?;
                    self.tail(tail)?;
                    Ok(())
                })
            })
            // args on separate lines
            .next(|| {
                self.has_vstruct(VStruct::BrokenIndent, || {
                    params(ListShape::Vertical)?;
                    self.fn_ret_ty(&fn_decl.output)?;
                    self.tail(tail)?;
                    Ok(())
                })
            })
            .result()
    }

    fn param(&self, param: &ast::Param, tail: Tail, _lcx: ListItemContext) -> FormatResult {
        self.with_attrs_tail(&param.attrs, param.span, tail, || {
            self.param_after_attrs(param, tail)
        })
    }

    fn param_after_attrs(&self, param: &ast::Param, tail: Tail) -> FormatResult {
        let colon_ty = |af: &Self| {
            af.out.token_space(":")?;
            af.ty_tail(&param.ty, tail)?;
            Ok(())
        };
        if let ast::PatKind::Ident(BindingMode(_, mutbl), ident, _) = param.pat.kind {
            match ident.name {
                // fn(TypeWithoutName)
                kw::Empty => return self.ty_tail(&param.ty, tail),
                kw::SelfLower => {
                    match param.ty.kind {
                        ast::TyKind::ImplicitSelf => {
                            self.mutability(mutbl)?;
                            self.ty_tail(&param.ty, tail)?;
                            return Ok(());
                        }
                        ast::TyKind::Ref(_, ref mut_ty) | ast::TyKind::PinnedRef(_, ref mut_ty)
                            if mut_ty.ty.kind.is_implicit_self() =>
                        {
                            return self.ty_tail(&param.ty, tail);
                        }
                        _ => {
                            self.mutability(mutbl)?;
                            self.out.token("self")?;
                            colon_ty(self)?;
                            return Ok(());
                        }
                    };
                }
                _ => {}
            }
        }
        let tail = if matches!(param.ty.kind, ast::TyKind::Infer) {
            tail
        } else {
            &self.tail_fn(colon_ty)
        };
        self.pat_tail(&param.pat, tail)?;
        Ok(())
    }

    fn fn_ret_ty(&self, output: &ast::FnRetTy) -> FormatResult {
        match output {
            ast::FnRetTy::Default(_) => {}
            ast::FnRetTy::Ty(ty) => {
                self.backtrack()
                    // todo recover width?
                    .next(|| self.out.space_token_space("->"))
                    .next(|| {
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        self.out.token_space("->")?;
                        Ok(())
                    })
                    .result()?;
                self.ty(ty)?;
            }
        }
        Ok(())
    }

    fn constness(&self, constness: ast::Const) -> FormatResult {
        match constness {
            ast::Const::Yes(_) => self.out.token_space("const"),
            ast::Const::No => Ok(()),
        }
    }

    fn extern_(&self, ext: &ast::Extern) -> FormatResult {
        match *ext {
            ast::Extern::None => {}
            ast::Extern::Implicit(_) => self.out.token_space("extern")?,
            ast::Extern::Explicit(ref abi, _) => {
                self.out.token_space("extern")?;
                self.strlit(abi)?;
                self.out.space()?;
            }
        }
        Ok(())
    }

    fn coroutine_kind(&self, coroutine_kind: &ast::CoroutineKind) -> FormatResult {
        match *coroutine_kind {
            ast::CoroutineKind::Async { .. } => self.out.token_space("async"),
            ast::CoroutineKind::Gen { .. } => self.out.token_space("gen"),
            ast::CoroutineKind::AsyncGen { .. } => {
                self.out.token_space("async")?;
                self.out.token_space("gen")?;
                Ok(())
            }
        }
    }
}
