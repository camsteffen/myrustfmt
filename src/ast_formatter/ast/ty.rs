use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::Braces;
use crate::ast_formatter::list::options::ListOptions;
use crate::ast_formatter::tail::Tail;
use crate::error::{FormatErrorKind, FormatResult};
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;

impl AstFormatter {
    pub fn ty(&self, ty: &ast::Ty) -> FormatResult {
        self.ty_tail(ty, None)
    }

    // todo breakpoints
    pub fn ty_tail(&self, ty: &ast::Ty, tail: Tail) -> FormatResult {
        let mut tail = Some(tail);
        let mut take_tail = || tail.take().unwrap();
        match &ty.kind {
            ast::TyKind::Array(ty, length) => {
                self.out.token("[")?;
                self.ty(ty)?;
                self.out.token_space(";")?;
                self.expr(&length.value)?;
                self.out.token("]")?;
            }
            ast::TyKind::BareFn(bare_fn_ty) => self.bare_fn_ty(bare_fn_ty)?,
            ast::TyKind::CVarArgs => self.out.token("...")?,
            ast::TyKind::ImplicitSelf => self.out.token("self")?,
            ast::TyKind::ImplTrait(_, bounds) => {
                self.out.token_space("impl")?;
                self.generic_bounds(bounds, take_tail())?;
            }
            ast::TyKind::Infer => self.out.token("_")?,
            ast::TyKind::MacCall(mac_call) => self.mac_call(mac_call)?,
            ast::TyKind::Never => self.out.token("!")?,
            ast::TyKind::Paren(ty) => {
                self.out.token("(")?;
                self.ty(ty)?;
                self.out.token(")")?;
            }
            ast::TyKind::Path(qself, path) => self.qpath(qself, path, false, take_tail())?,
            ast::TyKind::Ptr(mut_ty) => {
                self.out.token_space("*const")?;
                self.mut_ty(mut_ty)?
            }
            ast::TyKind::Ref(lifetime, mut_ty) => {
                self.out.token("&")?;
                if let Some(lifetime) = lifetime {
                    self.lifetime(lifetime)?;
                    self.out.space()?;
                }
                self.mut_ty(mut_ty)?;
            }
            ast::TyKind::Slice(elem) => {
                self.out.token("[")?;
                self.ty(elem)?;
                self.out.token("]")?;
            }
            ast::TyKind::TraitObject(bounds, syntax) => {
                match syntax {
                    ast::TraitObjectSyntax::Dyn => self.out.token_space("dyn")?,
                    ast::TraitObjectSyntax::DynStar => {
                        return Err(FormatErrorKind::UnsupportedSyntax.into())
                    }
                    ast::TraitObjectSyntax::None => {}
                }
                self.generic_bounds(bounds, take_tail())?;
            }
            ast::TyKind::Tup(elements) => {
                self.list(
                    Braces::Parens,
                    elements,
                    |af, ty, tail, _lcx| af.ty_tail(ty, tail),
                    ListOptions::new()
                        .force_trailing_comma(elements.len() == 1)
                        .tail(take_tail()),
                )?
            },
            ast::TyKind::Typeof(anon_const) => self.expr(&anon_const.value)?,
            ast::TyKind::Pat(..) | ast::TyKind::PinnedRef(..) | ast::TyKind::UnsafeBinder(..) => {
                return Err(FormatErrorKind::UnsupportedSyntax.into())
            }
            ast::TyKind::Dummy | ast::TyKind::Err(_) => panic!("unexpected TyKind"),
        }
        if let Some(tail) = tail {
            self.tail(tail)?;
        }
        Ok(())
    }

    pub fn lifetime(&self, lifetime: &ast::Lifetime) -> FormatResult {
        self.ident(lifetime.ident)
    }

    fn mut_ty(&self, mut_ty: &ast::MutTy) -> FormatResult {
        self.mutability(mut_ty.mutbl)?;
        self.ty(&mut_ty.ty)?;
        Ok(())
    }

    pub fn generic_bounds_optional(&self, bounds: &[ast::GenericBound]) -> FormatResult<bool> {
        if bounds.is_empty() {
            return Ok(false);
        }
        self.out.token(":")?;
        let indent_guard = self.space_or_wrap_indent_then(|| {
            // todo single line?
            self.generic_bounds(bounds, None)
        })?;
        if let Some(indent_guard) = indent_guard {
            indent_guard.close();
            self.out.newline_indent(VerticalWhitespaceMode::Break)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn generic_bounds(&self, bounds: &[ast::GenericBound], tail: Tail) -> FormatResult {
        self.simple_infix_chain("+", bounds, |b| self.generic_bound(b), true, tail)
    }

    fn generic_bound(&self, bound: &ast::GenericBound) -> FormatResult {
        match bound {
            ast::GenericBound::Outlives(lifetime) => self.lifetime(lifetime),
            ast::GenericBound::Trait(poly_trait_ref) => self.poly_trait_ref(poly_trait_ref),
            ast::GenericBound::Use(capture_args, _) => {
                self.out.token("use")?;
                self.list(
                    Braces::Angle,
                    capture_args,
                    |af, arg, tail, _lcx| af.precise_capturing_arg(arg, tail),
                    ListOptions::new(),
                )?;
                Ok(())
            }
        }
    }

    fn precise_capturing_arg(&self, arg: &ast::PreciseCapturingArg, tail: Tail) -> FormatResult {
        match arg {
            ast::PreciseCapturingArg::Arg(path, _) => self.path_tail(path, false, tail)?,
            ast::PreciseCapturingArg::Lifetime(lifetime) => {
                self.lifetime(lifetime)?;
                self.tail(tail)?;
            }
        }
        Ok(())
    }

    pub fn generic_arg(&self, arg: &ast::GenericArg, tail: Tail) -> FormatResult {
        match &arg {
            ast::GenericArg::Const(anon_const) => self.expr_tail(&anon_const.value, tail),
            ast::GenericArg::Lifetime(lifetime) => {
                self.lifetime(lifetime)?;
                self.tail(tail)?;
                Ok(())
            }
            ast::GenericArg::Type(ty) => self.ty_tail(ty, tail),
        }
    }

    pub fn trait_ref(&self, trait_ref: &ast::TraitRef) -> FormatResult {
        self.path(&trait_ref.path, false)
    }

    fn poly_trait_ref(&self, poly_trait_ref: &ast::PolyTraitRef) -> FormatResult {
        let ast::PolyTraitRef {
            bound_generic_params,
            modifiers,
            trait_ref,
            span: _,
        } = poly_trait_ref;
        if !bound_generic_params.is_empty() {
            self.out.token("for")?;
            self.generic_params(&poly_trait_ref.bound_generic_params)?;
            // todo wrap? comments?
            self.out.space()?;
        }
        let ast::TraitBoundModifiers {
            constness,
            asyncness,
            polarity,
        } = modifiers;
        match constness {
            ast::BoundConstness::Never => {}
            ast::BoundConstness::Always(_) => self.out.token_space("const")?,
            ast::BoundConstness::Maybe(_) => {
                self.out.token("~")?;
                self.out.token_space("const")?;
            }
        }
        match asyncness {
            ast::BoundAsyncness::Normal => {}
            ast::BoundAsyncness::Async(_) => self.out.token_space("async")?,
        }
        match polarity {
            ast::BoundPolarity::Positive => {}
            ast::BoundPolarity::Negative(_) => self.out.token("!")?,
            ast::BoundPolarity::Maybe(_) => self.out.token("?")?,
        }
        self.trait_ref(&trait_ref)?;
        Ok(())
    }
}
