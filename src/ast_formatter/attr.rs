use crate::ast_formatter::AstFormatter;
use crate::source_formatter::FormatResult;

use crate::ast_formatter::last_line::Tail;
use crate::ast_formatter::list::{list_overflow_yes, param_list_config};
use rustc_ast::ast;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

impl AstFormatter<'_> {
    pub fn attrs(&mut self, attrs: &[ast::Attribute]) -> FormatResult {
        for attr in attrs {
            self.attr(attr)?;
            self.out.newline_indent()?;
        }
        Ok(())
    }

    fn attr(&mut self, attr: &ast::Attribute) -> FormatResult {
        self.out.token_at("#", attr.span.lo())?;
        match attr.style {
            ast::AttrStyle::Inner => {
                self.out.token_expect("!")?;
            }
            ast::AttrStyle::Outer => {}
        }
        match attr.kind {
            ast::AttrKind::Normal(ref normal_attr) => match attr.meta() {
                None => todo!(),
                Some(meta) => {
                    self.out.token_expect("[")?;
                    self.meta_item(&meta)?;
                    self.out.token_expect("]")?;
                    Ok(())
                }
            },
            ast::AttrKind::DocComment(comment_kind, _symbol) => todo!(),
        }
    }

    pub fn meta_item(&mut self, meta: &ast::MetaItem) -> FormatResult {
        self.safety(&meta.unsafety)?;
        self.path(&meta.path)?;
        match &meta.kind {
            ast::MetaItemKind::Word => Ok(()),
            ast::MetaItemKind::List(items) => {
                let single_line_max_contents_width = RUSTFMT_CONFIG_DEFAULTS.attr_fn_like_width;
                self.list(
                    items,
                    |this, item| match item {
                        ast::MetaItemInner::MetaItem(item) => this.meta_item(item),
                        ast::MetaItemInner::Lit(lit) => this.meta_item_lit(lit),
                    },
                    param_list_config(Some(single_line_max_contents_width)),
                    list_overflow_yes(),
                    Tail::NONE,
                )
            }
            ast::MetaItemKind::NameValue(lit) => self.meta_item_lit(lit),
        }
    }

    fn meta_item_lit(&mut self, lit: &ast::MetaItemLit) -> FormatResult {
        todo!()
    }
}
