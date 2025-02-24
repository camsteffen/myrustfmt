use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{Braces, ListItemContext};
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;

use crate::ast_formatter::list::builder::list;
use crate::constraints::MultiLineShape;
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
        self.token_ident_generic_params("fn", item.ident, &generics)?;
        let is_block_after_decl = generics.where_clause.is_empty() && body.is_some();
        let param_list = list(Braces::PARENS, &sig.decl.inputs, Self::param);
        self.backtrack()
            .next(|| {
                self.with_single_line(|| {
                    param_list.format_single_line(self)?;
                    self.fn_ret_ty(&sig.decl.output)?;
                    if is_block_after_decl {
                        self.out.space_token("{")?;
                    }
                    Ok(())
                })
            })
            .otherwise(|| {
                param_list.format_separate_lines(self)?;
                self.fn_ret_ty(&sig.decl.output)?;
                if is_block_after_decl {
                    self.backtrack()
                        .next(|| self.out.space_token("{"))
                        .otherwise(|| {
                            self.out.newline_within_indent()?;
                            self.out.token("{")?;
                            Ok(())
                        })?;
                }
                Ok(())
            })?;
        self.where_clause(&generics.where_clause, body.is_some())?;
        if let Some(body) = body {
            if is_block_after_decl {
                self.block_separate_lines_after_open_brace(body)?;
            } else {
                self.block_separate_lines(body)?;
            }
        } else {
            self.out.token(";")?;
        }
        Ok(())
    }

    pub fn closure(&self, closure: &ast::Closure, tail: &Tail) -> FormatResult {
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
            ast::CaptureBy::Value { .. } => self.out.token_space("move")?,
        }
        self.constness(constness)?;
        if let Some(coroutine_kind) = coroutine_kind {
            self.coroutine_kind(coroutine_kind)?;
        }
        self.fn_decl(
            fn_decl,
            Braces::PIPE,
            &Tail::func(|af| {
                af.out.space()?;
                let can_remove_block = matches!(fn_decl.output, ast::FnRetTy::Default(_));
                af.closure_body(body, can_remove_block, tail)?;
                Ok(())
            }),
        )?;
        Ok(())
    }

    fn closure_body(&self, body: &ast::Expr, can_remove_block: bool, tail: &Tail) -> FormatResult {
        if can_remove_block {
            self.skip_single_expr_blocks_tail(body, tail, |body, tail| {
                // todo consider allowing `match`, `loop`, `if`, `for`, `while` if the header fits on one line
                //   should we preserve the block in such cases?
                //   actually, does indent-middle or SingleLineChains make sense here?
                //   and if/for/while should enforce single line headers

                // add a block unless it fits on a single line
                self.backtrack()
                    .next(|| {
                        self.constraints()
                            .with_multi_line_shape(MultiLineShape::BlockIndent, || {
                                self.expr_tail(body, tail)
                            })
                    })
                    .otherwise(|| {
                        self.expr_add_block(body)?;
                        self.tail(tail)?;
                        Ok(())
                    })
            })
        } else {
            self.expr_tail(body, tail)
        }
    }

    pub fn bare_fn_ty(&self, bare_fn_ty: &ast::BareFnTy) -> FormatResult {
        // todo
        // self.safety(&bare_fn_ty.safety)?;
        // self.extern_(&bare_fn_ty.ext)?;
        // self.generic_params(&bare_fn_ty.generic_params)?;
        self.out.token("fn")?;
        self.fn_decl(&bare_fn_ty.decl, Braces::PARENS, Tail::none())?;
        Ok(())
    }

    pub fn parenthesized_args(
        &self,
        parenthesized_args: &ast::ParenthesizedArgs,
        tail: &Tail,
    ) -> FormatResult {
        let (list_tail, final_tail) = match parenthesized_args.output {
            ast::FnRetTy::Default(_) => (tail, Tail::none()),
            ast::FnRetTy::Ty(_) => (Tail::none(), tail),
        };
        list(
            Braces::PARENS,
            &parenthesized_args.inputs,
            |af, ty, tail, _lcx| af.ty_tail(ty, tail),
        )
        .tail(list_tail)
        .format(self)?;
        self.fn_ret_ty(&parenthesized_args.output)?;
        // todo pass tail to ret ty?
        self.tail(final_tail)?;
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

    fn fn_decl(&self, fn_decl: &ast::FnDecl, braces: &'static Braces, tail: &Tail) -> FormatResult {
        let params = list(braces, &fn_decl.inputs, Self::param);
        let do_single_line = || {
            self.with_single_line(|| {
                params.format_single_line(self)?;
                self.fn_ret_ty(&fn_decl.output)?;
                Ok(())
            })?;
            self.tail(tail)?;
            Ok(())
        };
        if self.out.constraints().multi_line.get() < MultiLineShape::DisjointIndent {
            return do_single_line();
        }
        // args and return type all on one line
        self.backtrack()
            .next(do_single_line)
            // args on separate lines
            .otherwise(|| {
                params.format_separate_lines(self)?;
                self.fn_ret_ty(&fn_decl.output)?;
                self.tail(tail)?;
                Ok(())
            })
    }

    fn param(&self, param: &ast::Param, tail: &Tail, _lcx: ListItemContext) -> FormatResult {
        self.with_attrs_tail(&param.attrs, param.span, tail, || {
            self.param_after_attrs(param, tail)
        })
    }

    fn param_after_attrs(&self, param: &ast::Param, tail: &Tail) -> FormatResult {
        let colon_ty = |af: &Self| {
            af.out.token_space(":")?;
            af.ty_tail(&param.ty, tail)?;
            Ok(())
        };
        if let ast::PatKind::Ident(BindingMode(_, mutbl), ident, _) = param.pat.kind {
            match ident.name {
                // kw::Empty => return self.ty_tail(&param.ty, tail),
                kw::Empty => panic!(),
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
            &Tail::func(colon_ty)
        };
        self.pat_tail(&param.pat, tail)?;
        Ok(())
    }

    fn fn_ret_ty(&self, output: &ast::FnRetTy) -> FormatResult {
        match output {
            ast::FnRetTy::Default(_) => {}
            ast::FnRetTy::Ty(ty) => {
                self.backtrack()
                    .next(|| self.out.space_token_space("->"))
                    .otherwise(|| {
                        self.out.newline_within_indent()?;
                        self.out.token_space("->")?;
                        Ok(())
                    })?;
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
