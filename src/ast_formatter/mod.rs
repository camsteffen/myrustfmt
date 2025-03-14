use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::ast_module::AstModule;
use crate::config::Config;
use crate::constraints::Constraints;
use crate::error_emitter::{BufferedErrorEmitter, ErrorEmitter};
use crate::source_formatter::SourceFormatter;
use crate::error::FormatResult;
use crate::FormatModuleResult;
use crate::num::HPos;
use crate::whitespace::VerticalWhitespaceMode;

mod ast;
mod util;
mod list;
pub mod tail;
pub mod backtrack;

pub const INDENT_WIDTH: HPos = 4;

pub fn format_module(
    module: &AstModule,
    source: Rc<String>,
    path: Option<PathBuf>,
    config: &Config,
) -> FormatModuleResult {
    let constraints = Constraints::new(config.max_width);
    let errors = Rc::new(BufferedErrorEmitter::new(ErrorEmitter::new(path.clone())));
    let out = SourceFormatter::new(source, constraints, Rc::clone(&errors));
    let formatter = AstFormatter { errors, out };
    formatter.module(module, path.as_deref())
}

struct AstFormatter {
    errors: Rc<BufferedErrorEmitter>,
    out: SourceFormatter,
}

impl AstFormatter {
    fn module(self, module: &AstModule, path: Option<&Path>) -> FormatModuleResult {
        match self.do_module(module) {
            Err(e) => {
                // todo don't panic?
                // todo make it possible to panic inside ErrorEmitter instead?
                panic!(
                    "This is a bug :(\n{}",
                    e.display(self.out.source(), self.out.source_pos(), path)
                );
            }
            Ok(()) => FormatModuleResult {
                error_count: self.errors.error_count(),
                formatted: self.out.finish(),
            },
        }
    }

    fn do_module(&self, module: &AstModule) -> FormatResult {
        self.out.comments(VerticalWhitespaceMode::Top)?;
        self.with_attrs(&module.attrs, module.spans.inner_span, || {
            if let [until_last @ .., last] = &module.items[..] {
                for item in until_last {
                    self.item(item)?;
                    self.out.newline_indent(VerticalWhitespaceMode::Between)?;
                }
                self.item(last)?;
                self.out.newline(VerticalWhitespaceMode::Bottom)?;
            }
            Ok(())
        })?;
        Ok(())
    }
}
