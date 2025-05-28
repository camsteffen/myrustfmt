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

#[derive(Clone, Copy, PartialEq)]
pub enum FnDeclMode {
    BareFnTy,
    Closure,
    Fn { open_brace: bool },
}

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
            FnDeclMode::Fn {
                open_brace: is_block_after_decl,
            },
            &sig.decl,
            None,
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
                FnDeclMode::Closure,
                &closure.fn_decl,
                self.tail_fn(|af| {
                    af.out.space()?;
                    let has_return_type = match closure.fn_decl.output {
                        ast::FnRetTy::Default(_) => false,
                        ast::FnRetTy::Ty(_) => true,
                    };
                    let single_line_header = self.out.line() == first_line;
                    af.closure_body(&closure.body, has_return_type, single_line_header, tail)?;
                    Ok(())
                })
                .as_ref(),
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
                            | VStruct::List
                            | VStruct::NonBlockIndent,
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
        self.fn_decl(FnDeclMode::BareFnTy, &bare_fn_ty.decl, None)?;
        Ok(())
    }

    pub fn parenthesized_args(
        &self,
        parenthesized_args: &ast::ParenthesizedArgs,
        tail: Tail,
    ) -> FormatResult {
        let (list_tail, final_tail) = match parenthesized_args.output {
            ast::FnRetTy::Default(_) => (tail, None),
            ast::FnRetTy::Ty(_) => (None, tail),
        };
        self.list(
            Braces::Parens,
            &parenthesized_args.inputs,
            |af, ty, tail, _lcx| af.ty_tail(ty, tail),
            ListOptions::new().tail(list_tail),
        )?;
        if let ast::FnRetTy::Ty(_) = parenthesized_args.output {
            self.out.space()?;
            self.fn_ret_ty(&parenthesized_args.output)?;
        }
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

    fn fn_decl(&self, mode: FnDeclMode, fn_decl: &ast::FnDecl, tail: Tail) -> FormatResult {
        let args_braces = match mode {
            FnDeclMode::Closure => Braces::Pipe,
            _ => Braces::Parens,
        };
        let has_ret_ty = matches!(fn_decl.output, ast::FnRetTy::Ty(_));
        let open_brace_tail = |newline: bool| -> FormatResult {
            if mode == (FnDeclMode::Fn { open_brace: true }) {
                if newline {
                    self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                } else {
                    self.out.space()?;
                }
                self.out.token("{")?;
            }
            self.tail(tail)?;
            Ok(())
        };
        if fn_decl.inputs.is_empty() {
            self.enclosed_empty(args_braces)?;
            if !has_ret_ty {
                open_brace_tail(false)?;
                return Ok(());
            }
            self.backtrack()
                .next(|| {
                    self.space_could_wrap_indent(|| {
                        self.with_single_line(|| self.fn_ret_ty(&fn_decl.output))?;
                        open_brace_tail(false)?;
                        Ok(())
                    })
                })
                .next(|| {
                    self.indented(|| {
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        self.fn_ret_ty(&fn_decl.output)?;
                        Ok(())
                    })?;
                    open_brace_tail(true)?;
                    Ok(())
                })
                .result()?;
        } else {
            let with_args = |shape| {
                self.list(
                    args_braces,
                    &fn_decl.inputs,
                    Self::param,
                    ListOptions::new().shape(shape),
                )?;
                if has_ret_ty {
                    self.out.space()?;
                    self.fn_ret_ty(&fn_decl.output)?;
                }
                open_brace_tail(false)?;
                Ok(())
            };
            self.backtrack()
                .next(|| {
                    self.out.with_recover_width(|| {
                        self.with_single_line(|| with_args(ListShape::Horizontal))
                    })
                })
                .next(|| {
                    self.has_vstruct(VStruct::NonBlockIndent, || with_args(ListShape::Vertical))
                })
                .result()?
        }
        Ok(())
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
        let colon_ty_tail;
        let tail = if matches!(param.ty.kind, ast::TyKind::Infer) {
            tail
        } else {
            colon_ty_tail = self.tail_fn(colon_ty);
            colon_ty_tail.as_ref()
        };
        self.pat_tail(&param.pat, tail)?;
        Ok(())
    }

    fn fn_ret_ty(&self, output: &ast::FnRetTy) -> FormatResult {
        match output {
            ast::FnRetTy::Default(_) => {}
            ast::FnRetTy::Ty(ty) => {
                self.out.token_space("->")?;
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
