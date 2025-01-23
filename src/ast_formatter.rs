use crate::config::Config;
use crate::constraints::Constraints;
use crate::error::FormatResult;
use crate::error_emitter::ErrorEmitter;
use crate::source_formatter::SourceFormatter;
use rustc_ast::ast;
use std::path::PathBuf;

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
}

pub struct FormatCrateResult {
    pub source: String,
    pub formatted: String,
    pub exceeded_max_width: bool,
}

impl FormatCrateResult {
    pub fn expect_not_exceeded_max_width(&self) {
        if self.exceeded_max_width {
            panic!("Exceeded max width");
        }
    }
}

impl AstFormatter {
    pub fn new(source: String, path: Option<PathBuf>, config: Config) -> Self {
        let constraints = Constraints::new(config.max_width);
        let error_emitter = ErrorEmitter::new(path);
        let out = SourceFormatter::new(source, constraints, error_emitter);
        AstFormatter { config, out }
    }

    pub fn finish(self) -> FormatCrateResult {
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

    pub fn source(&self) -> &str {
        self.out.source()
    }
}
