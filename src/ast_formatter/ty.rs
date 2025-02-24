use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::Braces;
use crate::ast_formatter::list::builder::list;
use crate::ast_formatter::list::list_config::TupleListConfig;
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;
use rustc_ast::ast;

impl AstFormatter {
    pub fn ty(&self, ty: &ast::Ty) -> FormatResult {
        self.ty_tail(ty, Tail::none())
    }

    pub fn ty_tail(&self, ty: &ast::Ty, tail: &Tail) -> FormatResult {
        let mut tail = Some(tail);
        let mut take_tail = || tail.take().unwrap();
        match &ty.kind {
            ast::TyKind::Slice(elem) => {
                self.out.token("[")?;
                self.ty(elem)?;
                self.out.token("]")?;
            }
            ast::TyKind::Array(ty, length) => {
                self.out.token("[")?;
                self.ty(ty)?;
                self.out.token_space(";")?;
                self.anon_const(length)?;
                self.out.token("]")?;
            }
            ast::TyKind::Ptr(_mut_ty) => todo!(),
            ast::TyKind::Ref(lifetime, mut_ty) => {
                self.out.token("&")?;
                if let Some(lifetime) = lifetime {
                    self.lifetime(lifetime)?;
                    self.out.space()?;
                }
                self.mut_ty(mut_ty)?;
            }
            ast::TyKind::PinnedRef(_lifetime, _mut_ty) => todo!(),
            ast::TyKind::BareFn(bare_fn_ty) => self.bare_fn_ty(bare_fn_ty)?,
            ast::TyKind::Never => self.out.token("!")?,
            ast::TyKind::Tup(elements) => {
                list(Braces::PARENS, elements, |af, ty, tail, _lcx| {
                    af.ty_tail(ty, tail)
                })
                .config(TupleListConfig {
                    len: elements.len(),
                })
                .tail(take_tail())
                .format(self)?
            }
            ast::TyKind::Path(qself, path) => self.qpath(qself, path, false)?,
            ast::TyKind::TraitObject(bounds, syntax) => {
                match syntax {
                    ast::TraitObjectSyntax::Dyn => {
                        self.out.token_space("dyn")?;
                    }
                    ast::TraitObjectSyntax::DynStar => todo!(),
                    ast::TraitObjectSyntax::None => todo!(),
                }
                self.generic_bounds(bounds, take_tail())?;
            }
            ast::TyKind::ImplTrait(_, bounds) => {
                self.out.token_space("impl")?;
                self.generic_bounds(bounds, take_tail())?;
            }
            ast::TyKind::Paren(ty) => {
                self.out.token("(")?;
                self.ty(ty)?;
                self.out.token(")")?;
            }
            ast::TyKind::Typeof(anon_const) => self.anon_const(anon_const)?,
            ast::TyKind::Infer => self.out.token("_")?,
            ast::TyKind::ImplicitSelf => self.out.token("self")?,
            ast::TyKind::MacCall(_mac_call) => todo!(),
            ast::TyKind::CVarArgs => todo!(),
            ast::TyKind::Pat(_ty, _pat) => todo!(),
            ast::TyKind::UnsafeBinder(..) => todo!(),
            ast::TyKind::Dummy => todo!(),
            ast::TyKind::Err(_) => todo!(),
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
        self.backtrack()
            .next(|| {
                self.out.space()?;
                // todo single line?
                self.generic_bounds(bounds, Tail::none())?;
                Ok(false)
            })
            .otherwise(|| {
                self.indented(|| {
                    self.out.newline_within_indent()?;
                    self.generic_bounds(bounds, Tail::none())?;
                    Ok(())
                })?;
                self.out.newline_within_indent()?;
                Ok(true)
            })
    }

    pub fn generic_bounds(&self, bounds: &[ast::GenericBound], tail: &Tail) -> FormatResult {
        self.simple_infix_chain("+", bounds, |b| self.generic_bound(b), true, tail)
    }

    fn generic_bound(&self, bound: &ast::GenericBound) -> FormatResult {
        match bound {
            ast::GenericBound::Outlives(lifetime) => self.lifetime(lifetime),
            ast::GenericBound::Trait(poly_trait_ref) => self.poly_trait_ref(poly_trait_ref),
            ast::GenericBound::Use(_capture_args, _span) => todo!(),
        }
    }

    pub fn generic_arg(&self, arg: &ast::GenericArg, tail: &Tail) -> FormatResult {
        match &arg {
            ast::GenericArg::Const(anon_const) => self.anon_const_tail(anon_const, tail),
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
        for _param in &poly_trait_ref.bound_generic_params {
            todo!();
        }
        // poly_trait_ref.modifiers;
        self.trait_ref(&poly_trait_ref.trait_ref)?;
        Ok(())
    }
}
