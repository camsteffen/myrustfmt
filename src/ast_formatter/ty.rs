use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::Braces;
use crate::ast_formatter::list::list;
use crate::ast_formatter::list::list_config::ParamListConfig;
use crate::error::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter {
    pub fn ty(&self, ty: &ast::Ty) -> FormatResult {
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
            ast::TyKind::Never => todo!(),
            ast::TyKind::Tup(elements) => list(Braces::PARENS, elements, |ty| self.ty(ty))
                .config(&ParamListConfig {
                    single_line_max_contents_width: None,
                })
                .format(self)?,
            ast::TyKind::Path(qself, path) => self.qpath(qself, path, false)?,
            ast::TyKind::TraitObject(bounds, syntax) => {
                match syntax {
                    ast::TraitObjectSyntax::Dyn => {
                        self.out.token_space("dyn")?;
                    }
                    ast::TraitObjectSyntax::DynStar => todo!(),
                    ast::TraitObjectSyntax::None => todo!(),
                }
                self.generic_bounds(bounds)?;
            }
            ast::TyKind::ImplTrait(_, bounds) => {
                self.out.token_space("impl")?;
                self.generic_bounds(bounds)?;
            }
            ast::TyKind::Paren(ty) => {
                self.out.token("(")?;
                self.ty(ty)?;
                self.out.token(")")?;
            }
            ast::TyKind::Typeof(anon_const) => self.anon_const(anon_const)?,
            ast::TyKind::Infer => todo!(),
            ast::TyKind::ImplicitSelf => self.out.token("self")?,
            ast::TyKind::MacCall(_mac_call) => todo!(),
            ast::TyKind::CVarArgs => todo!(),
            ast::TyKind::Pat(_ty, _pat) => todo!(),
            ast::TyKind::Dummy => todo!(),
            ast::TyKind::Err(_) => todo!(),
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

    pub fn generic_bounds_optional(&self, bounds: &[ast::GenericBound]) -> FormatResult {
        if !bounds.is_empty() {
            self.out.token_space(":")?;
            self.generic_bounds(bounds)?;
        }
        Ok(())
    }

    pub fn generic_bounds(&self, bounds: &[ast::GenericBound]) -> FormatResult {
        self.infix_chain("+", bounds, |b| self.generic_bound(b), true)
    }

    fn generic_bound(&self, bound: &ast::GenericBound) -> FormatResult {
        match bound {
            ast::GenericBound::Outlives(lifetime) => self.lifetime(lifetime),
            ast::GenericBound::Trait(poly_trait_ref) => self.poly_trait_ref(poly_trait_ref),
            ast::GenericBound::Use(_capture_args, _span) => todo!(),
        }
    }

    pub fn generic_arg(&self, arg: &ast::GenericArg) -> FormatResult {
        match &arg {
            ast::GenericArg::Const(anon_const) => self.anon_const(anon_const),
            ast::GenericArg::Lifetime(lifetime) => self.lifetime(lifetime),
            ast::GenericArg::Type(ty) => self.ty(ty),
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
