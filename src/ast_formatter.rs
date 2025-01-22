use crate::config::Config;
use crate::constraints::Constraints;
use crate::error::FormatResult;
use crate::source_formatter::SourceFormatter;
use rustc_ast::ast;
use std::cell::Cell;

mod attr;
mod binary;
mod block;
mod common;
mod constraint_modifiers;
mod expr;
mod fallback;
mod r#fn;
mod generics;
mod item;
pub mod list;
mod local;
mod r#match;
mod pat;
mod path;
mod postfix_chain;
mod ty;
mod util;

pub struct AstFormatter {
    config: Config,
    out: SourceFormatter,
    /// True when there is a fallback routine planned for if the current routine produces code
    /// that does not meet the given constraints.
    has_fallback: Cell<bool>,
}

impl AstFormatter {
    pub fn new(source: impl Into<String>, config: Config) -> Self {
        let constraints = Constraints::new(config.max_width);
        let out = SourceFormatter::new(source.into(), constraints);
        AstFormatter {
            config,
            out,
            has_fallback: Cell::new(false),
        }
    }

    pub fn finish(self) -> String {
        self.out.finish()
    }

    pub fn crate_(&self, crate_: &ast::Crate) -> FormatResult {
        self.with_attrs(&crate_.attrs, crate_.spans.inner_span, || {
            for item in &crate_.items {
                self.item(item)?;
                self.out.newline_between_indent()?;
            }
            Ok(())
        })
    }

    fn config(&self) -> &Config {
        &self.config
    }

    pub fn pos(&self) -> usize {
        self.out.pos()
    }
}
