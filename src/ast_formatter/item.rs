use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::{ListConfig, ListWrapToFitConfig};
use crate::source_formatter::FormatResult;
use rustc_ast::ast;
use crate::ast_formatter::last_line::Tail;

impl<'a> AstFormatter<'a> {
    pub fn item(&mut self, item: &ast::Item) -> FormatResult {
        self.attrs(&item.attrs)?;
        self.vis(&item.vis)?;
        match &item.kind {
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
            },
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
            ast::ItemKind::Struct(_, _) => todo!(),
            ast::ItemKind::Union(_, _) => todo!(),
            ast::ItemKind::Trait(_) => todo!(),
            ast::ItemKind::TraitAlias(_, _) => todo!(),
            ast::ItemKind::Impl(_) => todo!(),
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
            },
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
                self.list(
                    items,
                    |this, (use_tree, _)| this.use_tree(use_tree),
                    UseTreeListConfig,
                    Tail::None
                )?
            },
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
        ListWrapToFitConfig::Yes { max_element_width: None }
    }
}