use rustc_ast::ast;
use rustc_span::symbol::Ident;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::config::{
    ListConfig, ListWrapToFitConfig, ParamListConfig, struct_field_list_config,
};
use crate::ast_formatter::list::{Braces, list};
use crate::config::Config;
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

impl<'a> AstFormatter {
    pub fn item(&self, item: &ast::Item) -> FormatResult {
        self.item_generic(item, |kind| self.item_kind(kind, item))
    }

    fn item_generic<K>(
        &self,
        item: &ast::Item<K>,
        kind: impl FnOnce(&K) -> FormatResult,
    ) -> FormatResult {
        self.attrs(&item.attrs)?;
        self.vis(&item.vis)?;
        kind(&item.kind)?;
        Ok(())
    }

    pub fn item_kind(&self, kind: &ast::ItemKind, item: &ast::Item) -> FormatResult {
        match kind {
            ast::ItemKind::ExternCrate(name) => {
                self.out.token_at("extern", item.span.lo())?;
                self.out.space()?;
                self.out.token_expect("crate")?;
                self.out.space()?;
                if name.is_some() {
                    self.out.eat_token()?;
                    self.out.space()?;
                    self.out.token_expect("as")?;
                    self.out.space()?;
                }
                self.ident(item.ident)?;
                self.out.token_end_at(";", item.span.hi())?;
            }
            ast::ItemKind::Use(use_tree) => {
                self.out.token_at_space("use", item.span.lo())?;
                self.use_tree(use_tree, Tail::SEMICOLON)?;
            }
            ast::ItemKind::Static(_) => todo!(),
            ast::ItemKind::Const(const_item) => self.const_item(const_item, item.ident)?,
            ast::ItemKind::Fn(fn_) => self.fn_(fn_, item)?,
            ast::ItemKind::Mod(safety, mod_kind) => {
                self.safety(safety)?;
                self.out.token_expect("mod")?;
                self.out.space()?;
                self.ident(item.ident)?;
                match mod_kind {
                    ast::ModKind::Loaded(items, ast::Inline::Yes, _mod_spans) => {
                        self.out.space()?;
                        self.block_generic(items, |item| self.item(item))?;
                    }
                    ast::ModKind::Loaded(_, ast::Inline::No, _) | ast::ModKind::Unloaded => {
                        self.out.token_end_at(";", item.span.hi())?;
                    }
                }
            }
            ast::ItemKind::ForeignMod(_) => todo!(),
            ast::ItemKind::GlobalAsm(_) => todo!(),
            ast::ItemKind::TyAlias(_) => todo!(),
            ast::ItemKind::Enum(def, generics) => self.enum_(&def.variants, generics, item)?,
            ast::ItemKind::Struct(variants, generics) => {
                self.struct_item(variants, generics, item)?
            }
            ast::ItemKind::Union(_, _) => todo!(),
            ast::ItemKind::Trait(trait_) => self.trait_(trait_, item)?,
            ast::ItemKind::TraitAlias(_, _) => todo!(),
            ast::ItemKind::Impl(impl_) => self.impl_(impl_, item)?,
            ast::ItemKind::MacCall(_) => todo!(),
            ast::ItemKind::MacroDef(_) => todo!(),
            ast::ItemKind::Delegation(_) => todo!(),
            ast::ItemKind::DelegationMac(_) => todo!(),
        }
        Ok(())
    }

    fn vis(&self, vis: &ast::Visibility) -> FormatResult {
        match vis.kind {
            ast::VisibilityKind::Public => {
                self.out.token_at("pub", vis.span.lo())?;
                self.out.space()?;
            }
            ast::VisibilityKind::Restricted {
                ref path,
                shorthand: _,
                ..
            } => {
                self.out.token_at("pub", vis.span.lo())?;
                self.out.space()?;
                self.path(path, false)?;
                self.out.space()?;
            }
            ast::VisibilityKind::Inherited => {}
        }
        Ok(())
    }

    fn const_item(&self, const_item: &ast::ConstItem, ident: Ident) -> FormatResult {
        self.out.token_expect("const")?;
        self.out.space()?;
        self.ident(ident)?;
        self.out.token_expect(":")?;
        self.out.space()?;
        self.ty(&const_item.ty)?;
        if let Some(expr) = &const_item.expr {
            self.out.space()?;
            self.out.token_expect("=")?;
            self.out.space()?;
            self.expr(expr)?;
        }
        self.out.token_expect(";")?;
        Ok(())
    }

    fn enum_(
        &self,
        variants: &[ast::Variant],
        generics: &ast::Generics,
        item: &ast::Item,
    ) -> FormatResult {
        self.out.token_expect("enum")?;
        self.out.space()?;
        self.ident(item.ident)?;
        self.generic_params(&generics.params)?;
        self.out.space()?;
        list(Braces::CURLY, variants, |v| self.variant(v)).format_separate_lines(self)?;
        Ok(())
    }

    fn variant(&self, variant: &ast::Variant) -> FormatResult {
        self.attrs(&variant.attrs)?;
        self.vis(&variant.vis)?;
        self.ident(variant.ident)?;
        self.variant_data(&variant.data)?;
        if let Some(_discriminant) = &variant.disr_expr {
            todo!()
        }
        Ok(())
    }

    fn impl_(&self, impl_: &ast::Impl, item: &ast::Item) -> FormatResult {
        let first_line = self.out.line();
        self.out.token_at("impl", item.span.lo())?;
        self.generic_params(&impl_.generics.params)?;
        self.single_line_or_line_break_indent_optional(self.out.line() == first_line, |broken| {
            if let Some(of_trait) = &impl_.of_trait {
                // self.with_single_line_optional(!broken, || self.trait_ref(of_trait))?;
                self.trait_ref(of_trait)?;
                self.line_break_fallback_with_optional_indent(!broken, |for_ty_broken| {
                    self.out.token_expect("for")?;
                    self.out.space()?;
                    self.with_single_line_optional(!for_ty_broken, || self.ty(&impl_.self_ty))?;
                    Ok(())
                })
            } else {
                self.ty(&impl_.self_ty)
                // self.with_single_line_optional(!broken, || self.ty(&impl_.self_ty))
            }
        })?;
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
        self.out.token_expect("type")?;
        self.out.space()?;
        self.ident(ident)?;
        if let Some(ty) = &ty_alias.ty {
            self.out.space()?;
            self.out.token_expect("=")?;
            self.out.space()?;
            self.ty(ty)?;
        }
        self.out.token_expect(";")?;
        Ok(())
    }

    fn struct_item(
        &self,
        variants: &ast::VariantData,
        generics: &ast::Generics,
        item: &ast::Item,
    ) -> FormatResult {
        self.out.token_expect("struct")?;
        self.out.space()?;
        self.ident(item.ident)?;
        self.generic_params(&generics.params)?;
        if !matches!(variants, ast::VariantData::Unit(_)) {
            self.variant_data(variants)?;
        }
        if matches!(
            variants,
            ast::VariantData::Unit(_) | ast::VariantData::Tuple(..)
        ) {
            self.out.token_expect(";")?;
        }
        Ok(())
    }

    fn trait_(&self, trait_: &ast::Trait, item: &ast::Item) -> FormatResult {
        self.out.token_expect("trait")?;
        self.out.space()?;
        self.ident(item.ident)?;
        // self.generic_params(&trait_.generics.params)?;
        // self.generic_bounds(&trait_.bounds)?;
        self.out.space()?;
        self.block_generic(&trait_.items, |item| self.assoc_item(item))?;
        Ok(())
    }

    fn variant_data(&self, variants: &ast::VariantData) -> FormatResult {
        match variants {
            ast::VariantData::Struct { fields, .. } => {
                self.out.space()?;
                list(Braces::CURLY, fields, |f| self.field_def(f))
                    .config(&struct_field_list_config(
                        false,
                        RUSTFMT_CONFIG_DEFAULTS.struct_variant_width,
                    ))
                    .format(self)?;
                Ok(())
            }
            ast::VariantData::Tuple(fields, _) => {
                list(Braces::PARENS, fields, |f| self.field_def(f))
                    .config(&ParamListConfig {
                        single_line_max_contents_width: None,
                    })
                    .format(self)
            }
            ast::VariantData::Unit(_) => Ok(()),
        }
    }

    fn field_def(&self, field: &ast::FieldDef) -> FormatResult {
        self.attrs(&field.attrs)?;
        self.vis(&field.vis)?;
        if let Some(ident) = field.ident {
            self.ident(ident)?;
            self.out.token_expect(":")?;
            self.out.space()?;
        }
        self.ty(&field.ty)?;
        Ok(())
    }

    fn use_tree(&self, use_tree: &ast::UseTree, tail: Tail<'_>) -> FormatResult {
        self.path(&use_tree.prefix, false)?;
        match use_tree.kind {
            ast::UseTreeKind::Simple(rename) => {
                if let Some(rename) = rename {
                    self.out.space()?;
                    self.out.token_expect("as")?;
                    self.out.space()?;
                    self.ident(rename)?;
                }
                self.tail(tail)?;
            }
            ast::UseTreeKind::Nested { ref items, span: _ } => {
                self.out.token_expect("::")?;
                let has_nested = items
                    .iter()
                    .any(|(item, _)| matches!(item.kind, ast::UseTreeKind::Nested { .. }));
                let list = list(Braces::CURLY_NO_PAD, items, |(use_tree, _)| {
                    self.use_tree(use_tree, Tail::NONE)
                })
                .config(&UseTreeListConfig)
                .tail(tail);
                if has_nested {
                    list.format_separate_lines(self)?
                } else {
                    list.format(self)?
                }
            }
            ast::UseTreeKind::Glob => todo!(),
        }
        Ok(())
    }
}

struct UseTreeListConfig;

impl ListConfig for UseTreeListConfig {
    fn single_line_reduce_max_width(&self, config: &Config) -> usize {
        if config.rustfmt_quirks { 2 } else { 0 }
    }

    fn wrap_to_fit() -> ListWrapToFitConfig {
        ListWrapToFitConfig::Yes {
            max_element_width: None,
        }
    }
}
