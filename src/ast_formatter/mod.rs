use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::ast_module::AstModule;
use crate::config::Config;
use crate::constraints::{Constraints, OwnedConstraints};
use crate::error_emitter::{BufferedErrorEmitter, ErrorEmitter};
use crate::source_formatter::SourceFormatter;
use crate::error::FormatResult;
use crate::FormatModuleResult;

mod ast;
mod util;
mod list;
pub mod tail;
pub mod backtrack;
mod whitespace;

pub const INDENT_WIDTH: u32 = 4;

pub fn format_module(
    module: &AstModule,
    source: Rc<String>,
    path: Option<PathBuf>,
    config: &Config,
) -> FormatModuleResult {
    let constraints = OwnedConstraints::new(Constraints::default(), Some(config.max_width));
    let error_emitter = Rc::new(BufferedErrorEmitter::new(ErrorEmitter::new(path.clone())));
    let out = SourceFormatter::new(source, constraints, Rc::clone(&error_emitter));
    let formatter = AstFormatter { error_emitter, out };
    formatter.module(module, path.as_deref())
}

struct AstFormatter {
    error_emitter: Rc<BufferedErrorEmitter>,
    out: SourceFormatter,
}

impl AstFormatter {
    fn module(self, module: &AstModule, path: Option<&Path>) -> FormatModuleResult {
        let result = (|| -> FormatResult {
            self.newline_top_if_comments()?;
            self.with_attrs(&module.attrs, module.spans.inner_span, || {
                if let [until_last @ .., last] = &module.items[..] {
                    for item in until_last {
                        self.item(item)?;
                        self.newline_between_indent()?;
                    }
                    self.item(last)?;
                    self.newline_bottom()?;
                }
                Ok(())
            })?;
            Ok(())
        })();
        match result {
            Err(e) => {
                // todo don't panic?
                // todo make it possible to panic inside ErrorEmitter instead?
                panic!(
                    "This is a bug :(\n{}",
                    e.display(self.out.source(), self.out.source_pos(), path)
                );
            }
            Ok(()) => FormatModuleResult {
                error_count: self.error_emitter.error_count(),
                formatted: self.out.finish(),
            },
        }
    }

    pub fn constraints(&self) -> &OwnedConstraints {
        self.out.constraints()
    }
}
