use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::brackets::Brackets;
use crate::ast_formatter::list::options::{
    FlexibleListStrategy, HorizontalListStrategy, ListOptions, ListStrategies,
};
use crate::ast_formatter::tail::Tail;
use crate::ast_utils::is_rustfmt_skip;
use crate::constraints::VStruct;
use crate::error::{FormatErrorKind, FormatResult, VerticalError};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use crate::span::Span;
use crate::util::cell_ext::CellExt;
use crate::whitespace::VerticalWhitespaceMode;
use rustc_ast::ast;

impl AstFormatter {
    pub fn with_attrs(
        &self,
        attrs: &[ast::Attribute],
        span: Span,
        format: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        self.with_attrs_tail(attrs, span, None, format)
    }

    // todo test usages
    // N.B the format function must emit the tail - this function emits the tail
    // only in cases where the format function is not used.
    pub fn with_attrs_tail(
        &self,
        attrs: &[ast::Attribute],
        span: Span,
        tail: Tail,
        format: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        self.has_vstruct_if(!attrs.is_empty(), VStruct::NonBlockIndent, || {
            // todo skip attributes as well?
            attrs.iter().try_for_each(|attr| self.attr(attr))?;
            // todo make my own attribute? or comment?
            // handle #[rustfmt::skip]
            let did_format = !attrs.iter().any(is_rustfmt_skip) && self.format_or_emit(format)?;
            if !did_format {
                let _guard = self.constraints().width_limit.replace_guard(None);
                self.out.copy_span(span)?;
                self.tail(tail)?;
            }
            Ok(())
        })
    }

    /// This is a "last resort" fallback for when a constraint error occurs, but we have no
    /// formatting strategy to try next. This means we have no way of formatting the user's code
    /// with the given constraints. So the error should be reported to the user, and we'll just copy
    /// the source as-is.
    fn format_or_emit(&self, format: impl FnOnce() -> FormatResult) -> FormatResult<bool> {
        let checkpoint = self.out.checkpoint_without_buffer_errors();
        let error_count_before = self.errors.error_count();
        let Err(err) = format() else { return Ok(true) };
        let (line, col) = self.out.line_col();
        match err.kind {
            // todo test all these outputs
            FormatErrorKind::Vertical(vertical)
            // todo propagate VStruct?
            | FormatErrorKind::VStruct {
                cause: vertical,
                ..
            } => match vertical {
                VerticalError::LineComment => self.errors.line_comment_not_allowed(line, col),
                VerticalError::MultiLineComment => {
                    self.errors.multi_line_comment_not_allowed(line, col)
                }
                // todo why return here?
                VerticalError::Newline => return Err(err),
            },
            FormatErrorKind::UnsupportedSyntax => {
                self.errors.unsupported_syntax(line, col);
            }
            // these are not expected
            FormatErrorKind::Logical | FormatErrorKind::WidthLimitExceeded => return Err(err),
        }
        assert!(
            self.errors.error_count() > error_count_before,
            "an error should be emitted before copy fallback",
        );
        self.out.restore_checkpoint(&checkpoint);
        Ok(false)
    }

    fn attr(&self, attr: &ast::Attribute) -> FormatResult {
        match attr.kind {
            // comments are handled by SourceFormatter
            ast::AttrKind::DocComment(_comment_kind, _symbol) => {}
            ast::AttrKind::Normal(_) => match attr.meta() {
                None => {
                    // todo do better, format key-value pairs
                    self.out.copy_span(attr.span.into())?;
                    self.out.newline_indent(VerticalWhitespaceMode::Break)?;
                }
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
                    match attr.style {
                        ast::AttrStyle::Inner => self.out.newline(VerticalWhitespaceMode::Between)?,
                        ast::AttrStyle::Outer => self.out.newline(VerticalWhitespaceMode::Break)?,
                    }
                    self.out.indent();
                }
            },
        }
        Ok(())
    }

    // todo tail?
    pub fn meta_item(&self, meta: &ast::MetaItem) -> FormatResult {
        self.safety(meta.unsafety)?;
        self.path(&meta.path, false)?;
        match &meta.kind {
            ast::MetaItemKind::Word => {}
            ast::MetaItemKind::List(items) => self.list(
                Brackets::Parens,
                items,
                |af, item, tail, _lcx| {
                    self.meta_item_inner(item)?;
                    af.tail(tail)?;
                    Ok(())
                },
                ListOptions {
                    strategies: ListStrategies::Flexible(FlexibleListStrategy {
                        horizontal: HorizontalListStrategy {
                            // todo test
                            contents_max_width: Some(RUSTFMT_CONFIG_DEFAULTS.attr_fn_like_width),
                            ..
                        },
                        ..
                    }),
                    ..
                },
            )?,
            ast::MetaItemKind::NameValue(lit) => {
                self.out.space_token_space("=")?;
                self.meta_item_lit(lit)?;
            }
        }
        Ok(())
    }

    pub fn meta_item_inner(&self, item: &ast::MetaItemInner) -> FormatResult {
        match item {
            ast::MetaItemInner::MetaItem(item) => self.meta_item(item)?,
            ast::MetaItemInner::Lit(lit) => self.meta_item_lit(lit)?,
        }
        Ok(())
    }

    fn meta_item_lit(&self, lit: &ast::MetaItemLit) -> FormatResult {
        self.out.copy_span(lit.span.into())
    }
}
