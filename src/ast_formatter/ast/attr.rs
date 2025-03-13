use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::list::Braces;
use crate::ast_formatter::list::builder::list;
use crate::ast_utils::is_rustfmt_skip;
use crate::error::{ConstraintErrorKind, FormatResult, FormatResultExt};
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;
use rustc_ast::ast;
use rustc_span::Span;
use crate::ast_formatter::tail::Tail;

impl AstFormatter {
    pub fn with_attrs(
        &self,
        attrs: &[ast::Attribute],
        span: Span,
        format: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        self.with_attrs_tail(attrs, span, Tail::none(), format)
    }

    // todo test usages
    // N.B the format function must emit the tail - this function emits the tail
    // only in cases where the format function is not used.
    pub fn with_attrs_tail(
        &self,
        attrs: &[ast::Attribute],
        span: Span,
        tail: &Tail,
        format: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        // todo skip attributes as well?
        attrs.iter().try_for_each(|attr| self.attr(attr))?;
        // todo make my own attribute? or comment?
        // handle #[rustfmt::skip]
        if attrs.iter().any(is_rustfmt_skip) {
            self.out
                .constraints()
                .with_max_width(None, || self.out.copy_span(span))?;
            self.tail(tail)?;
        } else if !self.out.has_any_constraint_recovery() {
            // todo don't do this in expr list item, when max width is not enforced
            self.with_copy_span_fallback(span, format, tail)?;
        } else {
            format()?;
        }
        Ok(())
    }

    /// This is a "last resort" fallback for when a constraint error occurs, but we have no
    /// formatting strategy to try next. This means we have no way of formatting the user's code
    /// with the given constraints. So the error should be reported to the user, and we'll just copy
    /// the source as-is.
    fn with_copy_span_fallback(
        &self,
        span: Span,
        format: impl FnOnce() -> FormatResult,
        tail: &Tail,
    ) -> FormatResult {
        // N.B. We're not using the typical `Checkpoint` here because we don't want to increment the
        // checkpoint counter which is only incremented when there is a valid formatting strategy to
        // fall back to.
        let checkpoint = self.out.checkpoint_without_buffer_errors();
        #[cfg(debug_assertions)]
        let error_count_before = self.error_emitter.error_count();
        let Err(e) = format().constraint_err_only()? else {
            return Ok(());
        };
        match e.kind {
            ConstraintErrorKind::NewlineNotAllowed => {
                let (line, col) = self.out.line_col();
                // todo emit a more appropriate error for bad comments
                self.error_emitter.newline_not_allowed(line, col);
            }
            // width limit errors are emitted before the error value is returned
            ConstraintErrorKind::WidthLimitExceeded => {}
            // unexpected
            ConstraintErrorKind::NextStrategy => return Err(e.into()),
        }
        #[cfg(debug_assertions)]
        assert!(self.error_emitter.error_count() > error_count_before, "an error should be emitted before copy fallback\nstack trace:\n{}", e.backtrace);
        self.out.restore_checkpoint(&checkpoint);
        self.out
            .constraints()
            .with_max_width(None, || self.out.copy_span(span))?;
        self.tail(tail)?;
        Ok(())
    }

    fn attr(&self, attr: &ast::Attribute) -> FormatResult {
        match attr.kind {
            // comments are handled by SourceFormatter
            ast::AttrKind::DocComment(_comment_kind, _symbol) => {}
            ast::AttrKind::Normal(_) => match attr.meta() {
                None => {
                    // todo do better, format key-value pairs
                    self.out.copy_span(attr.span)?;
                    self.newline_break_indent()?;
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
                        ast::AttrStyle::Inner => self.newline_between()?,
                        ast::AttrStyle::Outer => self.newline_break()?,
                    }
                    self.indent();
                }
            },
        }
        Ok(())
    }

    pub fn meta_item(&self, meta: &ast::MetaItem) -> FormatResult {
        self.safety(meta.unsafety)?;
        self.path(&meta.path, false)?;
        match &meta.kind {
            ast::MetaItemKind::Word => {}
            ast::MetaItemKind::List(items) => {
                list(Braces::PARENS, items, |af, item, tail, _lcx| {
                    match item {
                        ast::MetaItemInner::MetaItem(item) => af.meta_item(item)?,
                        ast::MetaItemInner::Lit(lit) => af.meta_item_lit(lit)?,
                    }
                    af.tail(tail)?;
                    Ok(())
                })
                .single_line_max_contents_width(RUSTFMT_CONFIG_DEFAULTS.attr_fn_like_width)
                .format(self)?
            }
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
