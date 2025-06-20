mod sort;
pub mod use_tree;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::options::{ListOptions, ListStrategies};
use crate::ast_formatter::list::{Braces, ListItemContext};
use crate::ast_formatter::tail::Tail;
use crate::error::{FormatErrorKind, FormatResult};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;
use rustc_ast::ptr::P;
use rustc_span::Symbol;
use rustc_span::symbol::Ident;

pub trait MaybeItem {
    fn as_item(&self) -> Option<&ast::Item>;
}

impl MaybeItem for P<ast::Item> {
    fn as_item(&self) -> Option<&ast::Item> {
        Some(self)
    }
}

impl MaybeItem for ast::Stmt {
    fn as_item(&self) -> Option<&ast::Item> {
        match &self.kind {
            ast::StmtKind::Item(item) => Some(item),
            _ => None,
        }
    }
}

impl AstFormatter {
    pub fn item(&self, item: &ast::Item) -> FormatResult {
        self.item_generic(item, |kind| self.item_kind(kind))
    }

    fn item_generic<K>(
        &self,
        item: &ast::Item<K>,
        kind: impl FnOnce(&K) -> FormatResult,
    ) -> FormatResult {
        self.with_attrs(&item.attrs, item.span, || {
            self.vis(&item.vis)?;
            kind(&item.kind)?;
            Ok(())
        })
    }

    pub fn item_kind(&self, kind: &ast::ItemKind) -> FormatResult {
        match *kind {
            ast::ItemKind::Const(ref const_item) => self.const_item(const_item)?,
            ast::ItemKind::Enum(ident, ref generics, ref def) => {
                self.enum_(ident, generics, &def.variants)?
            }
            ast::ItemKind::ExternCrate(name, ident) => self.extern_crate(name, ident)?,
            ast::ItemKind::Fn(ref fn_) => self.fn_(fn_)?,
            ast::ItemKind::ForeignMod(ref foreign_mod) => {
                self.out.token_space("extern")?;
                if let Some(abi) = foreign_mod.abi {
                    self.strlit(abi)?;
                    self.out.space()?;
                }
                self.block(false, &foreign_mod.items, |item| self.foreign_item(item))?;
            }
            ast::ItemKind::Impl(ref impl_) => self.impl_(impl_)?,
            ast::ItemKind::MacCall(ref mac_call) => {
                self.macro_call(mac_call)?;
                if !matches!(mac_call.args.delim, rustc_ast::token::Delimiter::Brace) {
                    self.out.token(";")?;
                }
            }
            // todo
            ast::ItemKind::MacroDef(..) => return Err(FormatErrorKind::UnsupportedSyntax.into()),
            ast::ItemKind::Mod(safety, ident, ref mod_kind) => {
                self.mod_item(safety, ident, mod_kind)?
            }
            ast::ItemKind::Static(ref static_item) => self.static_item(static_item)?,
            ast::ItemKind::Struct(ident, ref generics, ref variants) => {
                self.struct_or_union("struct", ident, variants, generics)?
            }
            ast::ItemKind::Trait(ref trait_) => self.trait_(trait_)?,
            ast::ItemKind::TyAlias(ref ty_alias) => {
                self.token_ident_generic_params("type", ty_alias.ident, &ty_alias.generics)?;
                if let Some(ty) = &ty_alias.ty {
                    self.out.space_token_space("=")?;
                    self.ty(ty)?;
                }
                self.out.token(";")?;
            }
            ast::ItemKind::Union(ident, ref generics, ref variants) => {
                self.struct_or_union("union", ident, variants, generics)?
            }
            ast::ItemKind::Use(ref use_tree) => {
                self.out.token_space("use")?;
                self.use_tree(use_tree, Some(&self.tail_token(";")))?;
            }
            ast::ItemKind::Delegation(_)
            | ast::ItemKind::DelegationMac(_)
            | ast::ItemKind::GlobalAsm(_)
            | ast::ItemKind::TraitAlias(..) => {
                return Err(FormatErrorKind::UnsupportedSyntax.into());
            }
        }
        Ok(())
    }

    fn vis(&self, vis: &ast::Visibility) -> FormatResult {
        match vis.kind {
            ast::VisibilityKind::Public => {
                self.out.token_space("pub")?;
            }
            ast::VisibilityKind::Restricted {
                ref path,
                shorthand: _,
                ..
            } => {
                self.out.token("pub")?;
                self.out.token("(")?;
                self.path(path, false)?;
                self.out.token(")")?;
                self.out.space()?;
            }
            ast::VisibilityKind::Inherited => {}
        }
        Ok(())
    }

    fn const_item(&self, const_item: &ast::ConstItem) -> FormatResult {
        self.out.token_space("const")?;
        self.ident(const_item.ident)?;
        self.out.token_space(":")?;
        let Some(expr) = &const_item.expr else {
            self.ty_tail(&const_item.ty, Some(&self.tail_token(";")))?;
            return Ok(());
        };
        self.ty(&const_item.ty)?;
        self.out.space_token_space("=")?;
        self.expr_tail(expr, Some(&self.tail_token(";")))?;
        Ok(())
    }

    fn enum_(
        &self,
        ident: Ident,
        generics: &ast::Generics,
        variants: &[ast::Variant],
    ) -> FormatResult {
        self.token_ident_generic_params("enum", ident, generics)?;
        self.out.space()?;
        self.list(
            Braces::Curly,
            variants,
            Self::variant,
            ListOptions {
                strategies: ListStrategies::vertical(),
                ..
            },
        )?;
        Ok(())
    }

    fn extern_crate(&self, name: Option<Symbol>, ident: Ident) -> FormatResult {
        self.out.token_space("extern")?;
        self.out.token_space("crate")?;
        if name.is_some() {
            self.out.copy_next_token()?;
            self.out.space_token_space("as")?;
        }
        self.ident(ident)?;
        self.out.token(";")?;
        Ok(())
    }

    fn mod_item(&self, safety: ast::Safety, ident: Ident, mod_kind: &ast::ModKind) -> FormatResult {
        self.safety(safety)?;
        self.out.token_space("mod")?;
        self.ident(ident)?;
        match mod_kind {
            ast::ModKind::Loaded(items, ast::Inline::Yes, ..) => {
                self.out.space()?;
                self.block_with_item_sorting(false, items, |item| self.item(item))?;
            }
            ast::ModKind::Loaded(_, ast::Inline::No, ..) | ast::ModKind::Unloaded => {
                self.out.token(";")?;
            }
        }
        Ok(())
    }

    fn variant(&self, variant: &ast::Variant, tail: Tail, _lcx: ListItemContext) -> FormatResult {
        self.with_attrs(&variant.attrs, variant.span, || {
            self.vis(&variant.vis)?;
            self.ident(variant.ident)?;
            self.variant_data(&variant.data, true, true)?;
            let Some(discriminant) = &variant.disr_expr else {
                return self.tail(tail);
            };
            self.assign_expr(&discriminant.value, tail)?;
            Ok(())
        })
    }

    fn impl_(&self, impl_: &ast::Impl) -> FormatResult {
        let first_line = self.out.line();
        self.out.token("impl")?;
        self.generic_params(&impl_.generics.params)?;
        let first_part = || match &impl_.of_trait {
            Some(of_trait) => self.trait_ref(of_trait),
            None => self.ty(&impl_.self_ty),
        };
        let indented = if self.out.line() == first_line {
            self.backtrack()
                .next(|| {
                    self.with_single_line(|| {
                        self.out.space()?;
                        first_part()?;
                        Ok(false)
                    })
                })
                .next(|| {
                    self.indented(|| {
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        first_part()?;
                        Ok(true)
                    })
                })
                .result()?
        } else {
            self.out.space()?;
            first_part()?;
            false
        };
        if impl_.of_trait.is_some() {
            self.backtrack()
                .next(|| {
                    self.space_could_wrap_indent(|| {
                        self.out.token_space("for")?;
                        self.ty(&impl_.self_ty)?;
                        Ok(())
                    })
                })
                .next(|| {
                    self.indented_optional(!indented, || {
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        self.out.token_space("for")?;
                        self.ty(&impl_.self_ty)?;
                        Ok(())
                    })
                })
                .result()?;
        }
        if !self.where_clause(&impl_.generics.where_clause, true)? {
            self.out.space()?;
        }
        self.block(false, &impl_.items, |item| self.assoc_item(item))?;
        Ok(())
    }

    fn assoc_item(&self, item: &ast::AssocItem) -> FormatResult {
        self.item_generic(item, |kind| {
            match kind {
                ast::AssocItemKind::Const(const_item) => self.const_item(const_item)?,
                ast::AssocItemKind::Fn(fn_) => self.fn_(fn_)?,
                ast::AssocItemKind::Type(ty_alias) => self.ty_alias(ty_alias)?,
                ast::AssocItemKind::MacCall(mac_call) => {
                    self.macro_call(mac_call)?;
                    self.out.token(";")?;
                }
                ast::AssocItemKind::Delegation(_) => {
                    return Err(FormatErrorKind::UnsupportedSyntax.into());
                }
                ast::AssocItemKind::DelegationMac(_) => {
                    return Err(FormatErrorKind::UnsupportedSyntax.into());
                }
            }
            Ok(())
        })
    }

    fn foreign_item(&self, foreign_item: &ast::ForeignItem) -> FormatResult {
        self.item_generic(foreign_item, |kind| {
            match kind {
                ast::ForeignItemKind::Fn(fn_) => self.fn_(fn_)?,
                ast::ForeignItemKind::MacCall(mac_call) => {
                    self.macro_call(mac_call)?;
                    self.out.token(";")?;
                }
                ast::ForeignItemKind::Static(static_item) => self.static_item(static_item)?,
                ast::ForeignItemKind::TyAlias(ty_alias) => self.ty_alias(ty_alias)?,
            }
            Ok(())
        })
    }

    fn ty_alias(&self, ty_alias: &ast::TyAlias) -> FormatResult {
        self.token_ident_generic_params("type", ty_alias.ident, &ty_alias.generics)?;
        self.generic_bounds_optional(&ty_alias.bounds)?;
        if let Some(ty) = &ty_alias.ty {
            self.out.space_token_space("=")?;
            self.ty(ty)?;
        }
        self.out.token(";")?;
        Ok(())
    }

    fn static_item(&self, static_item: &ast::StaticItem) -> FormatResult {
        self.out.token_space("static")?;
        self.ident(static_item.ident)?;
        self.out.token_space(":")?;
        self.ty(&static_item.ty)?;
        if let Some(expr) = &static_item.expr {
            self.out.space_token_space("=")?;
            self.expr(expr)?;
        }
        self.out.token(";")?;
        Ok(())
    }

    fn struct_or_union(
        &self,
        token: &'static str,
        ident: Ident,
        variants: &ast::VariantData,
        generics: &ast::Generics,
    ) -> FormatResult {
        let (has_body, has_semi) = match variants {
            ast::VariantData::Struct { .. } => (true, false),
            ast::VariantData::Tuple(..) => (true, true),
            ast::VariantData::Unit(_) => (false, true),
        };
        self.token_ident_generic_params(token, ident, generics)?;
        self.where_clause(&generics.where_clause, has_body)?;
        self.variant_data(variants, false, generics.where_clause.is_empty())?;
        if has_semi {
            self.out.token(";")?;
        }
        Ok(())
    }

    pub fn token_ident_generic_params(
        &self,
        token: &'static str,
        ident: Ident,
        generics: &ast::Generics,
    ) -> FormatResult {
        self.out.token_space(token)?;
        self.ident(ident)?;
        self.generic_params(&generics.params)?;
        Ok(())
    }

    fn trait_(&self, trait_: &ast::Trait) -> FormatResult {
        self.token_ident_generic_params("trait", trait_.ident, &trait_.generics)?;
        let wrapped_bounds = self.generic_bounds_optional(&trait_.bounds)?;
        // todo share this code with other constructs
        let has_where = self.where_clause(&trait_.generics.where_clause, true)?;
        let body = || self.block(false, &trait_.items, |item| self.assoc_item(item));
        if wrapped_bounds || has_where {
            body()?;
        } else {
            self.space_or_wrap_then(body)?;
        }
        Ok(())
    }

    fn variant_data(
        &self,
        variants: &ast::VariantData,
        is_enum: bool,
        is_same_line: bool,
    ) -> FormatResult {
        match variants {
            ast::VariantData::Struct { fields, .. } => {
                if is_same_line {
                    self.out.space()?;
                }
                self.list(
                    Braces::Curly,
                    fields,
                    Self::field_def,
                    ListOptions {
                        contents_max_width: Some(RUSTFMT_CONFIG_DEFAULTS.struct_variant_width),
                        is_struct: true,
                        strategies: if is_enum {
                            ListStrategies::flexible()
                        } else {
                            ListStrategies::vertical()
                        },
                        ..
                    },
                )?;
            }
            ast::VariantData::Tuple(fields, _) => {
                self.list(Braces::Parens, fields, Self::field_def, ListOptions { .. })?
            }
            ast::VariantData::Unit(_) => {}
        }
        Ok(())
    }

    fn field_def(&self, field: &ast::FieldDef, tail: Tail, _lcx: ListItemContext) -> FormatResult {
        self.with_attrs_tail(&field.attrs, field.span, tail, || {
            self.vis(&field.vis)?;
            if let Some(ident) = field.ident {
                self.ident(ident)?;
                self.out.token_space(":")?;
            }
            self.ty_tail(&field.ty, tail)?;
            if field.default.is_some() {
                // todo
                return Err(FormatErrorKind::UnsupportedSyntax.into());
            }
            Ok(())
        })
    }
}
