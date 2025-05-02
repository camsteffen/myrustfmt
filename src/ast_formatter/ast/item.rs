use rustc_ast::ast;
use rustc_span::Symbol;
use rustc_span::symbol::Ident;

use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{Braces, ListItemContext};
use crate::ast_formatter::list::options::{list_opt, ListShape, ListWrapToFit};
use crate::ast_formatter::tail::Tail;
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use crate::whitespace::VerticalWhitespaceMode;

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
        match *kind {
            ast::ItemKind::ExternCrate(name) => self.extern_crate(name, item)?,
            ast::ItemKind::Use(ref use_tree) => {
                self.out.token_space("use")?;
                self.use_tree(use_tree, &self.tail_token(";"))?;
            }
            ast::ItemKind::Static(ref static_item) => {
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
            ast::ItemKind::Const(ref const_item) => self.const_item(const_item, item.ident)?,
            ast::ItemKind::Fn(ref fn_) => self.fn_(fn_, item)?,
            ast::ItemKind::Mod(safety, ref mod_kind) => self.mod_item(safety, mod_kind, item)?,
            ast::ItemKind::ForeignMod(_) => todo!(),
            ast::ItemKind::GlobalAsm(_) => todo!(),
            ast::ItemKind::TyAlias(ref ty_alias) => {
                self.token_ident_generic_params("type", item.ident, &ty_alias.generics)?;
                if let Some(ty) = &ty_alias.ty {
                    self.out.space_token_space("=")?;
                    self.ty(ty)?;
                }
                self.out.token(";")?;
            }
            ast::ItemKind::Enum(ref def, ref generics) => {
                self.enum_(&def.variants, generics, item)?
            }
            ast::ItemKind::Struct(ref variants, ref generics) => {
                self.struct_item(variants, generics, item)?
            }
            ast::ItemKind::Union(_, _) => todo!(),
            ast::ItemKind::Trait(ref trait_) => self.trait_(trait_, item)?,
            ast::ItemKind::TraitAlias(_, _) => todo!(),
            ast::ItemKind::Impl(ref impl_) => self.impl_(impl_)?,
            ast::ItemKind::MacCall(ref mac_call) => {
                self.mac_call(mac_call)?;
                if !matches!(mac_call.args.delim, rustc_ast::token::Delimiter::Brace) {
                    self.out.token(";")?;
                }
            }
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
            self.ty_tail(&const_item.ty, &self.tail_token(";"))?;
            return Ok(());
        };
        self.ty(&const_item.ty)?;
        self.out.space_token_space("=")?;
        self.expr_tail(expr, &self.tail_token(";"))?;
        Ok(())
    }

    fn enum_(
        &self,
        variants: &[ast::Variant],
        generics: &ast::Generics,
        item: &ast::Item,
    ) -> FormatResult {
        self.token_ident_generic_params("enum", item.ident, generics)?;
        self.out.space()?;
        self.list(Braces::Curly, variants, Self::variant, list_opt().shape(ListShape::Vertical))?;
        Ok(())
    }

    fn extern_crate(&self, name: Option<Symbol>, item: &ast::Item) -> FormatResult {
        self.out.token_space("extern")?;
        self.out.token_space("crate")?;
        if name.is_some() {
            self.out.copy_next_token()?;
            self.out.space_token_space("as")?;
        }
        self.ident(item.ident)?;
        self.out.token(";")?;
        Ok(())
    }

    fn mod_item(
        &self,
        safety: ast::Safety,
        mod_kind: &ast::ModKind,
        item: &ast::Item,
    ) -> FormatResult {
        self.safety(safety)?;
        self.out.token_space("mod")?;
        self.ident(item.ident)?;
        match mod_kind {
            ast::ModKind::Loaded(items, ast::Inline::Yes, ..) => {
                self.out.space()?;
                self.block(items, |item| self.item(item))?;
            }
            ast::ModKind::Loaded(_, ast::Inline::No, ..) | ast::ModKind::Unloaded => {
                self.out.token(";")?;
            }
        }
        Ok(())
    }

    fn variant(&self, variant: &ast::Variant, tail: &Tail, _lcx: ListItemContext) -> FormatResult {
        self.with_attrs(&variant.attrs, variant.span, || {
            self.vis(&variant.vis)?;
            self.ident(variant.ident)?;
            self.variant_data(&variant.data, true, true)?;
            if let Some(_discriminant) = &variant.disr_expr {
                todo!()
            }
            self.tail(tail)?;
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
                .otherwise(|| {
                    self.indented(|| {
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
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
                        self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                        self.out.token_space("for")?;
                        self.ty(&impl_.self_ty)?;
                        Ok(())
                    })
                })?;
        }
        if !self.where_clause(&impl_.generics.where_clause, true)? {
            self.out.space()?;
        }
        self.block(&impl_.items, |item| self.assoc_item(item))?;
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
        self.token_ident_generic_params("type", ident, &ty_alias.generics)?;
        self.generic_bounds_optional(&ty_alias.bounds)?;
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
        let (has_body, has_semi) = match variants {
            ast::VariantData::Struct { .. } => (true, false),
            ast::VariantData::Tuple(..) => (true, true),
            ast::VariantData::Unit(_) => (false, true),
        };
        self.token_ident_generic_params("struct", item.ident, generics)?;
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

    fn trait_(&self, trait_: &ast::Trait, item: &ast::Item) -> FormatResult {
        self.token_ident_generic_params("trait", item.ident, &trait_.generics)?;
        let wrapped_bounds = self.generic_bounds_optional(&trait_.bounds)?;
        // todo share this code with other constructs
        let has_where = self.where_clause(&trait_.generics.where_clause, true)?;
        let body = || self.block(&trait_.items, |item| self.assoc_item(item));
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
                    list_opt()
                        .shape(if is_enum {
                            ListShape::Flexible
                        } else {
                            ListShape::Vertical
                        })
                        .single_line_max_contents_width(
                            RUSTFMT_CONFIG_DEFAULTS.struct_variant_width,
                        ),
                )?;
                Ok(())
            }
            ast::VariantData::Tuple(fields, _) => {
                self.list(Braces::Parens, fields, Self::field_def, list_opt())
            }
            ast::VariantData::Unit(_) => Ok(()),
        }
    }

    fn field_def(&self, field: &ast::FieldDef, tail: &Tail, _lcx: ListItemContext) -> FormatResult {
        self.with_attrs_tail(&field.attrs, field.span, tail, || {
            self.vis(&field.vis)?;
            if let Some(ident) = field.ident {
                self.ident(ident)?;
                self.out.token_space(":")?;
            }
            self.ty_tail(&field.ty, tail)?;
            Ok(())
        })
    }

    fn use_tree(&self, use_tree: &ast::UseTree, tail: &Tail) -> FormatResult {
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
                if let [(item, _)] = &items[..] {
                    self.out.skip_token("{")?;
                    self.use_tree(
                        item,
                        &self.tail_fn(|af| {
                            af.out.skip_token_if_present(",")?;
                            af.out.skip_token("}")?;
                            af.tail(tail)?;
                            Ok(())
                        }),
                    )?;
                } else {
                    self.list(
                        Braces::CurlyNoPad,
                        items,
                        |af, (use_tree, _), tail, _lcx| af.use_tree(use_tree, tail),
                        list_opt()
                            .item_requires_own_line(|(use_tree, _): &(ast::UseTree, _)| {
                                matches!(use_tree.kind, ast::UseTreeKind::Nested { .. })
                            })
                            .wrap_to_fit(ListWrapToFit::Yes { max_element_width: None })
                            .tail(tail),
                    )?;
                }
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
