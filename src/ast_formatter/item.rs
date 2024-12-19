use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::{
    ListConfig, ListWrapToFitConfig, StructFieldListConfig, list, param_list_config,
};
use crate::source_formatter::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter {
    pub fn item(&mut self, item: &ast::Item) -> FormatResult {
        self.item_generic(item, |this, kind| this.item_kind(kind, item))
    }

    fn item_generic<K>(
        &mut self,
        item: &ast::Item<K>,
        kind: impl FnOnce(&mut Self, &K) -> FormatResult,
    ) -> FormatResult {
        self.attrs(&item.attrs)?;
        self.vis(&item.vis)?;
        kind(self, &item.kind)?;
        Ok(())
    }

    pub fn item_kind(&mut self, kind: &ast::ItemKind, item: &ast::Item) -> FormatResult {
        match kind {
            ast::ItemKind::ExternCrate(name) => {
                self.out.token_at("extern", item.span.lo())?;
                self.out.space()?;
                self.out.token_expect("crate")?;
                self.out.space()?;
                self.ident(item.ident)?;
                self.out.token_end_at(";", item.span.hi())?;
            }
            ast::ItemKind::Use(use_tree) => {
                self.out.token_at_space("use", item.span.lo())?;
                self.use_tree(use_tree)?;
                self.out.token_end_at(";", item.span.hi())?;
            }
            ast::ItemKind::Static(_) => todo!(),
            ast::ItemKind::Const(_) => todo!(),
            ast::ItemKind::Fn(fn_) => self.fn_(fn_, item)?,
            ast::ItemKind::Mod(safety, mod_kind) => {
                self.safety(safety)?;
                self.out.token_expect("mod")?;
                self.out.space()?;
                self.ident(item.ident)?;
                match mod_kind {
                    ast::ModKind::Loaded(items, ast::Inline::Yes, _mod_spans) => {
                        self.out.space()?;
                        self.out.token_expect("{")?;
                        self.indented(|this| {
                            for item in items {
                                this.out.newline_indent()?;
                                this.item(item)?;
                            }
                            Ok(())
                        })?;
                        self.out.token_expect("}")?;
                    }
                    ast::ModKind::Loaded(_, ast::Inline::No, _) | ast::ModKind::Unloaded => {
                        self.out.token_end_at(";", item.span.hi())?;
                    }
                }
            }
            ast::ItemKind::ForeignMod(_) => todo!(),
            ast::ItemKind::GlobalAsm(_) => todo!(),
            ast::ItemKind::TyAlias(_) => todo!(),
            ast::ItemKind::Enum(_, _) => todo!(),
            ast::ItemKind::Struct(variants, generics) => {
                self.struct_item(variants, generics, item)?
            }
            ast::ItemKind::Union(_, _) => todo!(),
            ast::ItemKind::Trait(_) => todo!(),
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

    fn vis(&mut self, vis: &ast::Visibility) -> FormatResult {
        match vis.kind {
            ast::VisibilityKind::Public => {
                self.out.token_at("pub", vis.span.lo())?;
                self.out.space()?;
            }
            ast::VisibilityKind::Restricted {
                ref path,
                shorthand,
                ..
            } => {
                self.out.token_at("pub", vis.span.lo())?;
                self.out.space()?;
                self.path(path)?;
                self.out.space()?;
            }
            ast::VisibilityKind::Inherited => {}
        }
        Ok(())
    }

    fn impl_(&mut self, impl_: &ast::Impl, item: &ast::Item) -> FormatResult {
        self.out.token_at("impl", item.span.lo())?;
        self.generics(&impl_.generics)?;
        self.out.space()?;
        if let Some(of_trait) = &impl_.of_trait {
            self.trait_ref(of_trait)?;
            self.out.space()?;
            self.out.token_expect("for")?;
            self.out.space()?;
        }
        self.ty(&impl_.self_ty)?;
        self.out.space()?;
        self.out.token_expect("{")?;
        if !impl_.items.is_empty() {
            self.indented(|this| {
                for item in &impl_.items {
                    this.out.newline_indent()?;
                    this.item_generic(item, |this, kind| this.assoc_item_kind(kind, item))?;
                }
                Ok(())
            })?;
            self.out.newline_indent()?;
        }
        self.out.token_end_at("}", item.span.hi())?;
        Ok(())
    }

    fn assoc_item_kind(
        &mut self,
        kind: &ast::AssocItemKind,
        item: &ast::AssocItem,
    ) -> FormatResult {
        match kind {
            ast::AssocItemKind::Const(const_item) => todo!(),
            ast::AssocItemKind::Fn(fn_) => self.fn_(fn_, item),
            ast::AssocItemKind::Type(ty_alias) => todo!(),
            ast::AssocItemKind::MacCall(mac_call) => todo!(),
            ast::AssocItemKind::Delegation(delegation) => todo!(),
            ast::AssocItemKind::DelegationMac(delegation_mac) => todo!(),
        }
    }

    fn struct_item(
        &mut self,
        variants: &ast::VariantData,
        generics: &ast::Generics,
        item: &ast::Item,
    ) -> FormatResult {
        self.out.token_expect("struct")?;
        self.out.space()?;
        self.ident(item.ident)?;
        self.generics(generics)?;
        self.out.space()?;
        self.variant_data(variants)?;
        Ok(())
    }

    fn variant_data(&mut self, variants: &ast::VariantData) -> FormatResult {
        match variants {
            ast::VariantData::Struct { fields, .. } => {
                list(fields, Self::field_def, StructFieldListConfig).format(self)
            }
            ast::VariantData::Tuple(fields, _) => {
                list(fields, Self::field_def, param_list_config(None)).format(self)
            }
            ast::VariantData::Unit(_) => Ok(()),
        }
    }

    fn field_def(&mut self, field: &ast::FieldDef) -> FormatResult {
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

    fn use_tree(&mut self, use_tree: &ast::UseTree) -> FormatResult {
        self.path(&use_tree.prefix)?;
        match use_tree.kind {
            ast::UseTreeKind::Simple(None) => {}
            ast::UseTreeKind::Simple(Some(rename)) => {
                self.out.space()?;
                self.out.token_expect("as")?;
                self.out.space()?;
                self.ident(rename)?;
            }
            ast::UseTreeKind::Nested { ref items, span } => {
                self.out.token_expect("::")?;
                let has_nested = items
                    .iter()
                    .any(|(item, _)| matches!(item.kind, ast::UseTreeKind::Nested { .. }));
                if has_nested {
                    self.list_separate_lines(
                        items,
                        "{",
                        "}",
                        |this, (use_tree, _)| this.use_tree(use_tree),
                        Tail::NONE,
                    )?
                } else {
                    list(
                        items,
                        |this, (use_tree, _)| this.use_tree(use_tree),
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
