use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{list, param_list_config};
use crate::source_formatter::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter<'a> {
    pub fn ty(&mut self, ty: &ast::Ty) -> FormatResult {
        match &ty.kind {
            ast::TyKind::Slice(ty) => todo!(),
            ast::TyKind::Array(ty, length) => todo!(),
            ast::TyKind::Ptr(mut_ty) => todo!(),
            ast::TyKind::Ref(lifetime, mut_ty) => {
                self.out.token_at("&", ty.span.lo())?;
                if let Some(lifetime) = lifetime {
                    self.lifetime(lifetime)?;
                }
                self.mut_ty(mut_ty)?;
                Ok(())
            }
            ast::TyKind::PinnedRef(lifetime, mut_ty) => todo!(),
            ast::TyKind::BareFn(ty) => todo!(),
            ast::TyKind::Never => todo!(),
            ast::TyKind::Tup(elements) => {
                list(elements, |this, ty| this.ty(ty), param_list_config(None)).format(self)
            }
            ast::TyKind::Path(qself, path) => self.qpath(qself, path),
            ast::TyKind::TraitObject(bounds, syntax) => todo!(),
            ast::TyKind::ImplTrait(_, bounds) => {
                self.out.token_at("impl", ty.span.lo())?;
                self.out.space()?;
                self.generic_bounds(bounds)?;
                Ok(())
            }
            ast::TyKind::Paren(ty) => todo!(),
            ast::TyKind::Typeof(anon_const) => todo!(),
            ast::TyKind::Infer => todo!(),
            ast::TyKind::ImplicitSelf => self.out.token_at("self", ty.span.lo()),
            ast::TyKind::MacCall(mac_call) => todo!(),
            ast::TyKind::CVarArgs => todo!(),
            ast::TyKind::Pat(ty, pat) => todo!(),
            ast::TyKind::Dummy => todo!(),
            ast::TyKind::Err(_) => todo!(),
        }
    }

    pub fn lifetime(&mut self, lifetime: &ast::Lifetime) -> FormatResult {
        self.ident(lifetime.ident)
    }

    fn mut_ty(&mut self, mut_ty: &ast::MutTy) -> FormatResult {
        self.mutability(mut_ty.mutbl)?;
        self.ty(&mut_ty.ty)?;
        Ok(())
    }

    pub fn generic_bounds(&mut self, bounds: &[ast::GenericBound]) -> FormatResult {
        for bound in bounds {
            match bound {
                ast::GenericBound::Trait(poly_trait_ref) => self.poly_trait_ref(poly_trait_ref)?,
                ast::GenericBound::Outlives(lifetime) => todo!(),
                ast::GenericBound::Use(capture_args, span) => todo!(),
            }
        }
        Ok(())
    }

    pub fn trait_ref(&mut self, trait_ref: &ast::TraitRef) -> FormatResult {
        self.path(&trait_ref.path)
    }

    fn poly_trait_ref(&mut self, poly_trait_ref: &ast::PolyTraitRef) -> FormatResult {
        for param in &poly_trait_ref.bound_generic_params {
            todo!();
        }
        // poly_trait_ref.modifiers;
        self.trait_ref(&poly_trait_ref.trait_ref)?;
        Ok(())
    }
}
