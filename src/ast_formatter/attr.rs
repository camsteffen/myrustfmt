use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;

use crate::ast_formatter::list::config::ParamListConfig;
use crate::ast_formatter::list::{Braces, list};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;

impl AstFormatter {
    pub fn attrs(&self, attrs: &[ast::Attribute]) -> FormatResult {
        for attr in attrs {
            self.attr(attr)?;
        }
        Ok(())
    }

    fn attr(&self, attr: &ast::Attribute) -> FormatResult {
        match attr.kind {
            // comments are handled by SourceFormatter
            ast::AttrKind::DocComment(_comment_kind, _symbol) => Ok(()),
            ast::AttrKind::Normal(_) => match attr.meta() {
                None => todo!(),
                Some(meta) => {
                    self.out.token("#")?;
                    match attr.style {
                        ast::AttrStyle::Inner => {
                            self.out.token("!")?;
                        }
                        ast::AttrStyle::Outer => {}
                    }
                    self.out.token("[")?;
                    self.meta_item(&meta)?;
                    self.out.token("]")?;
                    self.out.newline_indent()?;
                    Ok(())
                }
            },
        }
    }

    pub fn meta_item(&self, meta: &ast::MetaItem) -> FormatResult {
        self.safety(&meta.unsafety)?;
        self.path(&meta.path, false)?;
        match &meta.kind {
            ast::MetaItemKind::Word => Ok(()),
            ast::MetaItemKind::List(items) => list(Braces::PARENS, items, |item| match item {
                ast::MetaItemInner::MetaItem(item) => self.meta_item(item),
                ast::MetaItemInner::Lit(lit) => self.meta_item_lit(lit),
            })
            .config(&ParamListConfig {
                single_line_max_contents_width: Some(RUSTFMT_CONFIG_DEFAULTS.attr_fn_like_width),
            })
            .overflow()
            .format(self),
            ast::MetaItemKind::NameValue(lit) => self.meta_item_lit(lit),
        }
    }

    fn meta_item_lit(&self, _lit: &ast::MetaItemLit) -> FormatResult {
        todo!()
    }
}
