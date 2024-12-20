use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{list, param_list_config};
use crate::error::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter {
    pub fn ty(&self, ty: &ast::Ty) -> FormatResult {
        match &ty.kind {
            ast::TyKind::Slice(_ty) => todo!(),
            ast::TyKind::Array(_ty, _length) => todo!(),
            ast::TyKind::Ptr(_mut_ty) => todo!(),
            ast::TyKind::Ref(lifetime, mut_ty) => {
                self.out.token_at("&", ty.span.lo())?;
                if let Some(lifetime) = lifetime {
                    self.lifetime(lifetime)?;
                    self.out.space()?;
                }
                self.mut_ty(mut_ty)?;
                Ok(())
            }
            ast::TyKind::PinnedRef(_lifetime, _mut_ty) => todo!(),
            ast::TyKind::BareFn(_ty) => todo!(),
            ast::TyKind::Never => todo!(),
            ast::TyKind::Tup(elements) => {
                list(elements, |ty| self.ty(ty), param_list_config(None)).format(self)
            }
            ast::TyKind::Path(qself, path) => self.qpath(qself, path),
            ast::TyKind::TraitObject(_bounds, _syntax) => todo!(),
            ast::TyKind::ImplTrait(_, bounds) => {
                self.out.token_at("impl", ty.span.lo())?;
                self.out.space()?;
                self.generic_bounds(bounds)?;
                Ok(())
            }
            ast::TyKind::Paren(_ty) => todo!(),
            ast::TyKind::Typeof(_anon_const) => todo!(),
            ast::TyKind::Infer => todo!(),
            ast::TyKind::ImplicitSelf => self.out.token_at("self", ty.span.lo()),
            ast::TyKind::MacCall(_mac_call) => todo!(),
            ast::TyKind::CVarArgs => todo!(),
            ast::TyKind::Pat(_ty, _pat) => todo!(),
            ast::TyKind::Dummy => todo!(),
            ast::TyKind::Err(_) => todo!(),
        }
    }

    pub fn lifetime(&self, lifetime: &ast::Lifetime) -> FormatResult {
        self.ident(lifetime.ident)
    }

    fn mut_ty(&self, mut_ty: &ast::MutTy) -> FormatResult {
        self.mutability(mut_ty.mutbl)?;
        self.ty(&mut_ty.ty)?;
        Ok(())
    }

    pub fn generic_bounds(&self, bounds: &[ast::GenericBound]) -> FormatResult {
        for bound in bounds {
            match bound {
                ast::GenericBound::Trait(poly_trait_ref) => self.poly_trait_ref(poly_trait_ref)?,
                ast::GenericBound::Outlives(_lifetime) => todo!(),
                ast::GenericBound::Use(_capture_args, _span) => todo!(),
            }
        }
        Ok(())
    }

    pub fn trait_ref(&self, trait_ref: &ast::TraitRef) -> FormatResult {
        self.path(&trait_ref.path)
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
