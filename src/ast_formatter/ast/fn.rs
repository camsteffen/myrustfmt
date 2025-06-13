use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{Braces, ListItemContext};
use crate::ast_formatter::tail::Tail;
use crate::error::{FormatErrorKind, FormatResult};

use crate::ast_formatter::list::options::{ListOptions, ListShape};
use crate::constraints::VStruct;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::BindingMode;
use rustc_ast::ast;
use rustc_span::symbol::kw;

impl AstFormatter {
    pub fn fn_(&self, fn_: &ast::Fn) -> FormatResult {
        let ast::Fn {
            generics,
            sig,
            body,
            ..
        } = fn_;
        self.fn_header(&sig.header)?;
        self.token_ident_generic_params("fn", fn_.ident, generics)?;
        let is_block_after_decl = generics.where_clause.is_empty() && body.is_some();
        let wrapped_return_type = self.fn_decl(
            &sig.decl,
            Braces::Parens,
            Some(&self.tail_fn(|af| {
                if is_block_after_decl {
                    af.out.space_allow_newlines()?;
                    af.out.token("{")?;
                }
                Ok(())
            })),
        )?;
        if is_block_after_decl && wrapped_return_type {
            self.out.newline_indent(VerticalWhitespaceMode::Break)?;
            self.out.token("{")?;
        }
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
                ast::ClosureBinder::For { .. } => {
                    return Err(FormatErrorKind::UnsupportedSyntax.into())
                }
                ast::ClosureBinder::NotPresent => {}
            }
            match closure.capture_clause {
                ast::CaptureBy::Ref => {}
                ast::CaptureBy::Use { .. } => return Err(FormatErrorKind::UnsupportedSyntax.into()),
                ast::CaptureBy::Value { .. } => self.out.token_space("move")?,
            }
            self.constness(closure.constness)?;
            if let Some(coroutine_kind) = &closure.coroutine_kind {
                self.coroutine_kind(coroutine_kind)?;
            }
            let body = |af: &AstFormatter| -> FormatResult {
                let has_return_type = match closure.fn_decl.output {
                    ast::FnRetTy::Default(_) => false,
                    ast::FnRetTy::Ty(_) => true,
                };
                let single_line_header = af.out.line() == first_line;
                self.allow_vstructs(VStruct::Block | VStruct::Match, || {
                    af.closure_body(&closure.body, has_return_type, single_line_header, tail)
                })?;
                Ok(())
            };
            let wrapped_return_type = self.fn_decl(
                &closure.fn_decl,
                Braces::Pipe,
                Some(&self.tail_fn(|af| {
                    af.out.space_allow_newlines()?;
                    body(af)?;
                    Ok(())
                })),
            )?;
            if wrapped_return_type {
                self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                body(self)?;
            }
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
            // it must be a block
            return self.expr_tail(body, tail);
        }
        if !single_line_header {
            self.expr_force_plain_block(body)?;
            self.tail(tail)?;
            return Ok(());
        }
        self.skip_single_expr_blocks_tail(body, tail, |body, tail| {
            self.backtrack()
                .next(|| {
                    self.could_wrap_indent(|| {
                        let disallowed_vstructs = VStruct::Closure
                            | VStruct::ControlFlow
                            | VStruct::List
                            | VStruct::NonBlockIndent;
                        self.disallow_vstructs(disallowed_vstructs, || self.expr_tail(body, tail))
                    })
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
        self.fn_decl(&bare_fn_ty.decl, Braces::Parens, None)?;
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
            self.fn_ret_ty(&parenthesized_args.output, None)?;
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

    /// N.B. `tail` is not invoked if the return type is wrapped (`true` is returned)
    fn fn_decl(
        &self,
        fn_decl: &ast::FnDecl,
        braces: Braces,
        tail: Tail,
    ) -> FormatResult</*wrapped_return_type*/bool> {
        let has_ret_ty = matches!(fn_decl.output, ast::FnRetTy::Ty(_));
        let return_ty_tail = |single_line: bool, force_wrap: bool| -> FormatResult<bool> {
            if !has_ret_ty {
                self.tail(tail)?;
                return Ok(false);
            }
            let indent_guard = self.begin_indent();
            let wrapped = if force_wrap {
                self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                true
            } else {
                self.out.space_allow_newlines()?
            };
            if wrapped {
                self.fn_ret_ty(&fn_decl.output, None)?;
            } else {
                indent_guard.close();
                self.with_single_line_if(single_line, || self.fn_ret_ty(&fn_decl.output, tail))?;
            }
            Ok(wrapped)
        };
        let args = |arg_list_shape| {
            self.has_vstruct(VStruct::NonBlockIndent, || {
                self.list(
                    braces,
                    &fn_decl.inputs,
                    Self::param,
                    ListOptions::new().shape(arg_list_shape),
                )
            })
        };

        // if there are no args, then we might wrap the return type to recover width
        if fn_decl.inputs.is_empty() {
            let first_line = self.out.line();
            args(ListShape::Horizontal)?;
            if !has_ret_ty {
                self.tail(tail)?;
                return Ok(false);
            }
            if self.out.line() > first_line {
                return return_ty_tail(false, false);
            }
            return self
                .backtrack()
                .next(|| self.out.with_recover_width(|| return_ty_tail(true, false)))
                .next(|| return_ty_tail(false, true))
                .result();
        }

        // there are one or more args
        self.backtrack()
            .next(|| {
                self.out.with_recover_width(|| {
                    args(ListShape::Horizontal)?;
                    return_ty_tail(true, false)
                })
            })
            .next(|| {
                args(ListShape::Vertical)?;
                return_ty_tail(false, false)
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
        // fn(TypeWithoutName)
        if let ast::PatKind::Missing = param.pat.kind {
            return self.ty_tail(&param.ty, tail);
        }
        if let ast::PatKind::Ident(BindingMode(_, mutbl), ident, _) = param.pat.kind
            && ident.name == kw::SelfLower
        {
            match param.ty.kind {
                ast::TyKind::ImplicitSelf => {
                    self.mutability(mutbl)?;
                    self.ty_tail(&param.ty, tail)?;
                }
                ast::TyKind::Ref(_, ref mut_ty) | ast::TyKind::PinnedRef(_, ref mut_ty)
                    if mut_ty.ty.kind.is_implicit_self() =>
                {
                    self.ty_tail(&param.ty, tail)?;
                }
                _ => {
                    self.mutability(mutbl)?;
                    self.out.token("self")?;
                    colon_ty(self)?;
                }
            }
            return Ok(());
        }
        let colon_ty_tail;
        let tail = if matches!(param.ty.kind, ast::TyKind::Infer) {
            tail
        } else {
            colon_ty_tail = self.tail_fn(colon_ty);
            Some(&colon_ty_tail)
        };
        self.pat_tail(&param.pat, tail)?;
        Ok(())
    }

    fn fn_ret_ty(&self, output: &ast::FnRetTy, tail: Tail) -> FormatResult {
        match output {
            ast::FnRetTy::Default(_) => {}
            ast::FnRetTy::Ty(ty) => {
                self.out.token_space("->")?;
                self.ty_tail(ty, tail)?;
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
            ast::Extern::Explicit(abi, _) => {
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
