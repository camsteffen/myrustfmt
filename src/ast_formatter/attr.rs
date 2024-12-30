use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::list_config::ParamListConfig;
use crate::ast_formatter::list::{Braces, list};
use crate::ast_formatter::util::tail::Tail;
use crate::ast_utils::is_rustfmt_skip;
use crate::error::FormatResult;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use rustc_span::Span;

impl AstFormatter {
    // todo test usages
    pub fn with_attrs(
        &self,
        attrs: &[ast::Attribute],
        span: Span,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        self.with_attrs_tail(attrs, span, Tail::NONE, f)
    }

    pub fn with_attrs_tail(
        &self,
        attrs: &[ast::Attribute],
        span: Span,
        tail: &Tail,
        f: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        // todo skip attributes as well?
        self.attrs(attrs)?;
        if attrs.iter().any(is_rustfmt_skip) {
            self.out.constraints().with_no_max_width(|| {
                self.out.copy_span(span)?;
                self.tail(tail)?;
                Ok(())
            })?;
        } else {
            f()?;
        }
        Ok(())
    }

    fn attrs(&self, attrs: &[ast::Attribute]) -> FormatResult {
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
            ast::MetaItemKind::Word => {}
            ast::MetaItemKind::List(items) => list(Braces::PARENS, items, |item| match item {
                ast::MetaItemInner::MetaItem(item) => self.meta_item(item),
                ast::MetaItemInner::Lit(lit) => self.meta_item_lit(lit),
            })
            .config(&ParamListConfig {
                single_line_max_contents_width: Some(RUSTFMT_CONFIG_DEFAULTS.attr_fn_like_width),
            })
            .overflow()
            .format(self)?,
            ast::MetaItemKind::NameValue(lit) => {
                self.out.space_token_space("=")?;
                self.meta_item_lit(lit)?;
            }
        }
        Ok(())
    }

    fn meta_item_lit(&self, lit: &ast::MetaItemLit) -> FormatResult {
        self.out.copy_span(lit.span)
    }
}
