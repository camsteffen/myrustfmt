use std::cell::RefCell;
use std::error::Error;
use crate::ast_module::AstModule;
use crate::config::Config;
use crate::constraints::{CheckpointCounter, Constraints, OwnedConstraints};
use crate::error_emitter::ErrorEmitter;
use crate::source_formatter::SourceFormatter;
use std::path::PathBuf;
use std::rc::Rc;
use crate::error::FormatResult;

mod ast;
mod checkpoint;
mod util;
mod list;
pub mod tail;
pub mod backtrack;

pub const INDENT_WIDTH: u32 = 4;

#[derive(Debug)]
pub struct FormatModuleResult {
    pub error_count: u32,
    pub formatted: String,
}

impl FormatModuleResult {
    pub fn into_result(self) -> Result<String, Box<dyn Error>> {
        let Self {
            error_count,
            formatted,
        } = self;
        if error_count > 0 {
            return Err(
                format!("Some errors occurred. Formatted:\n{}", formatted)
                    .into(),
            );
        }
        Ok(formatted)
    }

    pub fn expect_no_errors(self) -> String {
        let Self {
            error_count,
            formatted,
        } = self;
        assert_eq!(error_count, 0, "Some errors occurred. Formatted:\n{}", formatted);
        formatted
    }
}

pub fn format_module(
    module: &AstModule,
    source: Rc<String>,
    path: Option<PathBuf>,
    config: &Config,
) -> FormatModuleResult {
    let constraints = OwnedConstraints(RefCell::new(Rc::new(Constraints::new(config.max_width))));
    let error_emitter = Rc::new(ErrorEmitter::new(path));
    let out = SourceFormatter::new(source, constraints, Rc::clone(&error_emitter));
    let formatter = AstFormatter { error_emitter, out };
    formatter.module(module)
}

struct AstFormatter {
    error_emitter: Rc<ErrorEmitter>,
    out: SourceFormatter,
}

impl AstFormatter {
    fn module(self, module: &AstModule) -> FormatModuleResult {
        let result = (|| -> FormatResult {
            self.out.newline_above_if_comments()?;
            self.with_attrs(&module.attrs, module.spans.inner_span, || {
                if let [until_last @ .., last] = &module.items[..] {
                    for item in until_last {
                        self.item(item)?;
                        self.out.newline_between_indent()?;
                    }
                    self.item(last)?;
                    self.out.newline_below()?;
                }
                Ok(())
            })?;
            Ok(())
        })();
        match result {
            Err(e) => {
                self.error_emitter
                    .fatal_format_error(e, self.out.source(), self.out.pos())
            }
            Ok(()) => FormatModuleResult {
                error_count: self.error_emitter.error_count(),
                formatted: self.out.finish(),
            },
        }
    }

    pub fn checkpoint_counter(&self) -> &Rc<CheckpointCounter> {
        self.out.checkpoint_counter()
    }

    pub fn constraints(&self) -> &OwnedConstraints {
        self.out.constraints()
    }
}
