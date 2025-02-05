use rustc_ast::ast;
use rustc_span::symbol::Ident;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::list_config::{
    ListConfig, ListWrapToFitConfig, ParamListConfig, struct_field_list_config,
};
use crate::ast_formatter::list::{Braces, ListItemConfig, list};
use crate::ast_formatter::util::tail::Tail;
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

impl AstFormatter {
    pub fn item(&self, item: &ast::Item) -> FormatResult {
        self.item_generic(item, |kind| self.item_kind(kind, item))
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

    pub fn item_kind(&self, kind: &ast::ItemKind, item: &ast::Item) -> FormatResult {
        match kind {
            ast::ItemKind::ExternCrate(name) => {
                self.out.token_space("extern")?;
                self.out.token_space("crate")?;
                if name.is_some() {
                    self.out.copy_next_token()?;
                    self.out.space_token_space("as")?;
                }
                self.ident(item.ident)?;
                self.out.token(";")?;
            }
            ast::ItemKind::Use(use_tree) => {
                self.out.token_space("use")?;
                self.use_tree_tail(use_tree, &Tail::token(";"))?;
            }
            ast::ItemKind::Static(static_item) => {
                self.out.token_space("static")?;
                self.ident(item.ident)?;
                self.out.token_space(":")?;
                self.ty(&static_item.ty)?;
                if let Some(expr) = &static_item.expr {
                    self.out.space_token_space("=")?;
                    self.expr(expr)?;
                }
                self.out.token(";")?;
            }
            ast::ItemKind::Const(const_item) => self.const_item(const_item, item.ident)?,
            ast::ItemKind::Fn(fn_) => self.fn_(fn_, item)?,
            ast::ItemKind::Mod(safety, mod_kind) => {
                self.safety(safety)?;
                self.out.token_space("mod")?;
                self.ident(item.ident)?;
                match mod_kind {
                    ast::ModKind::Loaded(items, ast::Inline::Yes, _mod_spans) => {
                        self.out.space()?;
                        self.block_generic(items, |item| self.item(item))?;
                    }
                    ast::ModKind::Loaded(_, ast::Inline::No, _) | ast::ModKind::Unloaded => {
                        self.out.token(";")?;
                    }
                }
            }
            ast::ItemKind::ForeignMod(_) => todo!(),
            ast::ItemKind::GlobalAsm(_) => todo!(),
            ast::ItemKind::TyAlias(ty_alias) => {
                self.out.token_space("type")?;
                self.ident(item.ident)?;
                self.generic_params(&ty_alias.generics.params)?;
                if let Some(ty) = &ty_alias.ty {
                    self.out.space_token_space("=")?;
                    self.ty(ty)?;
                }
                self.out.token(";")?;
            }
            ast::ItemKind::Enum(def, generics) => self.enum_(&def.variants, generics, item)?,
            ast::ItemKind::Struct(variants, generics) => {
                self.struct_item(variants, generics, item)?
            }
            ast::ItemKind::Union(_, _) => todo!(),
            ast::ItemKind::Trait(trait_) => self.trait_(trait_, item)?,
            ast::ItemKind::TraitAlias(_, _) => todo!(),
            ast::ItemKind::Impl(impl_) => self.impl_(impl_)?,
            ast::ItemKind::MacCall(mac_call) => {
                self.mac_call(mac_call)?;
                self.out.token(";")?;
            },
            // todo
            ast::ItemKind::MacroDef(_) => self.out.copy_span(item.span)?,
            ast::ItemKind::Delegation(_) => todo!(),
            ast::ItemKind::DelegationMac(_) => todo!(),
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

    fn const_item(&self, const_item: &ast::ConstItem, ident: Ident) -> FormatResult {
        self.out.token_space("const")?;
        self.ident(ident)?;
        self.out.token_space(":")?;
        let Some(expr) = &const_item.expr else {
            self.ty_tail(&const_item.ty, &Tail::token(";"))?;
            return Ok(());
        };
        self.ty(&const_item.ty)?;
        self.out.space_token_space("=")?;
        self.expr_tail(expr, &Tail::token(";"))?;
        Ok(())
    }

    fn enum_(
        &self,
        variants: &[ast::Variant],
        generics: &ast::Generics,
        item: &ast::Item,
    ) -> FormatResult {
        self.out.token_space("enum")?;
        self.ident(item.ident)?;
        self.generic_params(&generics.params)?;
        self.out.space()?;
        list(Braces::CURLY, variants, |v| self.variant(v)).format_separate_lines(self)?;
        Ok(())
    }

    fn variant(&self, variant: &ast::Variant) -> FormatResult {
        self.with_attrs(&variant.attrs, variant.span, || {
            self.vis(&variant.vis)?;
            self.ident(variant.ident)?;
            self.variant_data(&variant.data, true)?;
            if let Some(_discriminant) = &variant.disr_expr {
                todo!()
            }
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
                .next_single_line(|| {
                    self.out.space()?;
                    first_part()?;
                    Ok(false)
                })
                .otherwise(|| {
                    self.indented(|| {
                        self.out.newline_within_indent()?;
                        first_part()?;
                        Ok(true)
                    })
                })?
        } else {
            self.out.space()?;
            first_part()?;
            false
        };
        if impl_.of_trait.is_some() {
            self.backtrack()
                .next(|| {
                    self.out.space()?;
                    self.out.token_space("for")?;
                    self.ty(&impl_.self_ty)?;
                    Ok(())
                })
                .otherwise(|| {
                    self.indented_optional(!indented, || {
                        self.out.newline_within_indent()?;
                        self.out.token_space("for")?;
                        self.ty(&impl_.self_ty)?;
                        Ok(())
                    })
                })?;
        }
        self.where_clause(&impl_.generics.where_clause)?;
        if impl_.generics.where_clause.is_empty() {
            self.out.space()?;
        }
        self.block_generic(&impl_.items, |item| self.assoc_item(item))?;
        Ok(())
    }

    fn assoc_item(&self, item: &ast::AssocItem) -> FormatResult {
        self.item_generic(item, |kind| self.assoc_item_kind(kind, item))
    }

    fn assoc_item_kind(&self, kind: &ast::AssocItemKind, item: &ast::AssocItem) -> FormatResult {
        match kind {
            ast::AssocItemKind::Const(const_item) => self.const_item(const_item, item.ident),
            ast::AssocItemKind::Fn(fn_) => self.fn_(fn_, item),
            ast::AssocItemKind::Type(ty_alias) => self.ty_alias(ty_alias, item.ident),
            ast::AssocItemKind::MacCall(_mac_call) => todo!(),
            ast::AssocItemKind::Delegation(_delegation) => todo!(),
            ast::AssocItemKind::DelegationMac(_delegation_mac) => todo!(),
        }
    }

    fn ty_alias(&self, ty_alias: &ast::TyAlias, ident: Ident) -> FormatResult {
        self.out.token_space("type")?;
        self.ident(ident)?;
        if let Some(ty) = &ty_alias.ty {
            self.out.space_token_space("=")?;
            self.ty(ty)?;
        }
        self.out.token(";")?;
        Ok(())
    }

    fn struct_item(
        &self,
        variants: &ast::VariantData,
        generics: &ast::Generics,
        item: &ast::Item,
    ) -> FormatResult {
        self.out.token_space("struct")?;
        self.ident(item.ident)?;
        self.generic_params(&generics.params)?;
        if !matches!(variants, ast::VariantData::Unit(_)) {
            self.variant_data(variants, false)?;
        }
        if matches!(
            variants,
            ast::VariantData::Unit(_) | ast::VariantData::Tuple(..)
        ) {
            self.out.token(";")?;
        }
        Ok(())
    }

    fn trait_(&self, trait_: &ast::Trait, item: &ast::Item) -> FormatResult {
        self.out.token_space("trait")?;
        self.ident(item.ident)?;
        // self.generic_params(&trait_.generics.params)?;
        self.generic_bounds_optional(&trait_.bounds)?;
        self.out.space()?;
        self.block_generic(&trait_.items, |item| self.assoc_item(item))?;
        Ok(())
    }

    fn variant_data(&self, variants: &ast::VariantData, is_enum: bool) -> FormatResult {
        match variants {
            ast::VariantData::Struct { fields, .. } => {
                self.out.space()?;
                let list = list(Braces::CURLY, fields, |f| self.field_def(f)).config(
                    struct_field_list_config(RUSTFMT_CONFIG_DEFAULTS.struct_variant_width),
                );
                if is_enum {
                    list.format(self)?;
                } else {
                    list.format_separate_lines(self)?;
                }
                Ok(())
            }
            ast::VariantData::Tuple(fields, _) => {
                list(Braces::PARENS, fields, |f| self.field_def(f))
                    .config(ParamListConfig {
                        single_line_max_contents_width: None,
                    })
                    .format(self)
            }
            ast::VariantData::Unit(_) => Ok(()),
        }
    }

    fn field_def(&self, field: &ast::FieldDef) -> FormatResult {
        self.with_attrs(&field.attrs, field.span, || {
            self.vis(&field.vis)?;
            if let Some(ident) = field.ident {
                self.ident(ident)?;
                self.out.token_space(":")?;
            }
            self.ty(&field.ty)?;
            Ok(())
        })
    }

    fn use_tree(&self, use_tree: &ast::UseTree) -> FormatResult {
        self.use_tree_tail(use_tree, Tail::none())
    }

    fn use_tree_tail(&self, use_tree: &ast::UseTree, tail: &Tail) -> FormatResult {
        self.path(&use_tree.prefix, false)?;
        match use_tree.kind {
            ast::UseTreeKind::Simple(rename) => {
                if let Some(rename) = rename {
                    self.out.space_token_space("as")?;
                    self.ident(rename)?;
                }
                self.tail(tail)?;
            }
            ast::UseTreeKind::Nested { ref items, span: _ } => {
                self.out.token("::")?;
                list(Braces::CURLY_NO_PAD, items, |(use_tree, _)| {
                    self.use_tree(use_tree)
                })
                .config(UseTreeListConfig)
                .item_config(UseTreeListItemConfig)
                .tail(tail)
                .format(self)?;
            }
            ast::UseTreeKind::Glob => {
                self.out.token("::")?;
                self.out.token("*")?;
                self.tail(tail)?;
            }
        }
        Ok(())
    }
}

struct UseTreeListConfig;

impl ListConfig for UseTreeListConfig {
    fn wrap_to_fit() -> ListWrapToFitConfig {
        ListWrapToFitConfig::Yes {
            max_element_width: None,
        }
    }
}

#[derive(Clone, Copy)]
struct UseTreeListItemConfig;
impl ListItemConfig for UseTreeListItemConfig {
    type Item = (ast::UseTree, ast::NodeId);

    const ITEMS_POSSIBLY_MUST_HAVE_OWN_LINE: bool = true;

    fn item_must_have_own_line((item, _): &Self::Item) -> bool {
        matches!(item.kind, ast::UseTreeKind::Nested { .. })
    }
}
