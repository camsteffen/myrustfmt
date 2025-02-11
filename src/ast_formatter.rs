use crate::ast_module::AstModule;
use crate::config::Config;
use crate::constraints::Constraints;
use crate::error_emitter::ErrorEmitter;
use crate::source_formatter::SourceFormatter;
use std::path::PathBuf;
use std::rc::Rc;

mod attr;
mod backtrack;
mod binary;
mod block;
mod checkpoint;
mod common;
mod constraint_modifiers;
mod expr;
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
    error_emitter: Rc<ErrorEmitter>,
    out: SourceFormatter,
}

// todo rename?
pub struct FormatModuleResult {
    pub formatted: String,
    pub exceeded_max_width: bool,
}

impl FormatModuleResult {
    pub fn expect_not_exceeded_max_width(self) -> String {
        if self.exceeded_max_width {
            panic!("Exceeded max width");
        }
        self.formatted
    }
}

impl AstFormatter {
    pub fn new(source: Rc<String>, path: Option<PathBuf>, config: Rc<Config>) -> Self {
        let constraints = Constraints::new(config.max_width);
        let error_emitter = Rc::new(ErrorEmitter::new(path));
        let out = SourceFormatter::new(source, constraints, Rc::clone(&error_emitter));
        AstFormatter { error_emitter, out }
    }

    pub fn module(self, module: &AstModule) -> FormatModuleResult {
        let result = self.with_attrs(&module.attrs, module.spans.inner_span, || {
            if let [until_last @ .., last] = &module.items[..] {
                for item in until_last {
                    self.item(item)?;
                    self.out.newline_between_indent()?;
                }
                self.item(last)?;
                self.out.newline_below()?;
            }
            Ok(())
        });
        match result {
            Ok(()) => self.out.finish(),
            Err(e) => {
                self.error_emitter
                    .fatal_format_error(e, self.out.source(), self.out.pos())
            }
        }
    }
}
