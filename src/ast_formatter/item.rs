use rustc_ast::ast;
use rustc_ast::ptr::P;
use rustc_span::Span;
use rustc_span::symbol::Ident;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{
    ListConfig, ListWrapToFitConfig, list, param_list_config, struct_field_list_config,
};
use crate::error::FormatResult;

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
                self.use_tree(use_tree)?;
                self.out.token_end_at(";", item.span.hi())?;
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
                        self.list_separate_lines(items, "{", "}", |item| self.item(item))?;
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
        self.out.newline_indent()?;
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
        self.list_separate_lines(variants, "{", "}", |v| self.variant(v))?;
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
        self.out.token_at("impl", item.span.lo())?;
        self.generic_params(&impl_.generics.params)?;
        self.out.space()?;
        if let Some(of_trait) = &impl_.of_trait {
            self.trait_ref(of_trait)?;
            self.out.space()?;
            self.out.token_expect("for")?;
            self.out.space()?;
        }
        self.ty(&impl_.self_ty)?;
        self.where_clause(&impl_.generics.where_clause)?;
        if impl_.generics.where_clause.is_empty() {
            self.out.space()?;
        }
        self.assoc_items(&impl_.items, item.span)?;
        Ok(())
    }

    fn assoc_items(&self, items: &[P<ast::AssocItem>], item_span: Span) -> FormatResult {
        self.out.token_expect("{")?;
        if !items.is_empty() {
            self.indented(|| {
                for item in items {
                    self.out.newline_indent()?;
                    self.assoc_item(item)?;
                }
                Ok(())
            })?;
            self.out.newline_indent()?;
        }
        self.out.token_end_at("}", item_span.hi())?;
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
            self.out.space()?;
            self.variant_data(variants)?;
        }
        if matches!(variants, ast::VariantData::Unit(_) | ast::VariantData::Tuple(..)) {
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
        self.assoc_items(&trait_.items, item.span)?;
        Ok(())
    }

    fn variant_data(&self, variants: &ast::VariantData) -> FormatResult {
        match variants {
            ast::VariantData::Struct { fields, .. } => list(
                fields,
                |f| self.field_def(f),
                struct_field_list_config(false),
            )
            .format(self),
            ast::VariantData::Tuple(fields, _) => {
                list(fields, |f| self.field_def(f), param_list_config(None)).format(self)
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

    fn use_tree(&self, use_tree: &ast::UseTree) -> FormatResult {
        self.path(&use_tree.prefix, false)?;
        match use_tree.kind {
            ast::UseTreeKind::Simple(None) => {}
            ast::UseTreeKind::Simple(Some(rename)) => {
                self.out.space()?;
                self.out.token_expect("as")?;
                self.out.space()?;
                self.ident(rename)?;
            }
            ast::UseTreeKind::Nested { ref items, span: _ } => {
                self.out.token_expect("::")?;
                let has_nested = items
                    .iter()
                    .any(|(item, _)| matches!(item.kind, ast::UseTreeKind::Nested { .. }));
                if has_nested {
                    self.list_separate_lines(items, "{", "}", |(use_tree, _)| {
                        self.use_tree(use_tree)
                    })?
                } else {
                    list(
                        items,
                        |(use_tree, _)| self.use_tree(use_tree),
                        UseTreeListConfig,
                    )
                    .format(self)?
                }
            }
            ast::UseTreeKind::Glob => todo!(),
        }
        Ok(())
    }
}

struct UseTreeListConfig;

impl ListConfig for UseTreeListConfig {
    const START_BRACE: &'static str = "{";
    const END_BRACE: &'static str = "}";
    const PAD_CONTENTS: bool = false;

    fn wrap_to_fit() -> ListWrapToFitConfig {
        ListWrapToFitConfig::Yes {
            max_element_width: None,
        }
    }
}
