use rustc_span::SourceFile;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use crate::FormatModuleResult;
use crate::ast_module::AstModule;
use crate::config::Config;
use crate::error::{FormatResult, error_formatting_at};
use crate::error_emitter::{BufferedErrorEmitter, ErrorEmitter};
use crate::num::HSize;
use crate::source_formatter::SourceFormatter;
use crate::whitespace::VerticalWhitespaceMode;

mod ast;
pub mod backtrack;
mod list;
pub mod tail;
pub mod util;

pub const INDENT_WIDTH: HSize = 4;

pub fn format_module(
    module: &AstModule,
    source_file: SourceFile,
    path: Option<PathBuf>,
    config: &Config,
) -> FormatModuleResult {
    let errors = Rc::new(
        BufferedErrorEmitter::new(ErrorEmitter::new(path.clone(), config.capture_error_output)),
    );
    // todo need Arc?
    let out = SourceFormatter::new(
        path,
        Arc::new(source_file),
        Rc::clone(&errors),
        config.max_width,
    );
    let formatter = AstFormatter { errors, out };
    formatter.module(module)
}

struct AstFormatter {
    errors: Rc<BufferedErrorEmitter>,
    out: SourceFormatter,
}

impl AstFormatter {
    fn module(self, module: &AstModule) -> FormatModuleResult {
        match self.do_module(module) {
            Err(e) => {
                // todo don't panic?
                // todo make it possible to panic inside ErrorEmitter instead?
                if cfg!(debug_assertions) {
                    self.out.debug_buffer();
                }
                panic!(
                    "This is a bug :(\n{}",
                    e.display(
                        self.out.source_reader.source(),
                        self.out.source_reader.pos(),
                        self.out.source_reader.path.as_deref(),
                    )
                );
            }
            Ok(()) => {
                let Self { errors, out } = self;
                let formatted = out.finish();
                let (error_count, error_output) = Rc::into_inner(errors).unwrap().finish();
                FormatModuleResult {
                    error_count,
                    error_output,
                    formatted,
                }
            }
        }
    }

    fn do_module(&self, module: &AstModule) -> FormatResult {
        self.out.comments(VerticalWhitespaceMode::Top)?;
        // todo skip the whole file if there's a skip attribute?
        self.with_attrs(&module.attrs, module.spans.inner_span, || {
            self.list_with_item_sorting(&module.items, |item| self.item(item))
        })?;
        self.out.newline(VerticalWhitespaceMode::Bottom)?;
        Ok(())
    }

    // todo use or delete
    // todo make it a macro? looks innocuous
    #[allow(unused)]
    fn bug(&self) -> ! {
        panic!(
            "{}",
            error_formatting_at(
                self.out.source_reader.source(),
                self.out.source_reader.pos(),
                self.out.source_reader.path.as_deref(),
            )
        );
    }
}
