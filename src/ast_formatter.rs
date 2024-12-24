use crate::source_formatter::SourceFormatter;
use std::cell::Cell;
use crate::config::Config;
use crate::error::FormatResult;

mod attr;
mod block;
mod common;
mod dot_chain;
mod expr;
mod fallback;
mod r#fn;
mod generics;
mod item;
mod last_line;
pub mod list;
mod local;
mod pat;
mod path;
mod ty;
mod constraint_modifiers;
mod wrapping;
mod infix_chain;
mod binary;
mod r#match;

pub struct AstFormatter {
    config: Config,
    out: SourceFormatter,
    allow_multiline_overflow: Cell<bool>,
}

impl<'a> AstFormatter {
    pub fn new(config: Config, out: SourceFormatter) -> Self {
        AstFormatter {
            config,
            out,
            allow_multiline_overflow: Cell::new(true),
        }
    }

    pub fn finish(self) -> String {
        self.out.finish()
    }

    pub fn crate_(&self, crate_: &rustc_ast::ast::Crate) -> FormatResult {
        self.attrs(&crate_.attrs)?;
        for item in &crate_.items {
            self.item(item)?;
            self.out.newline_indent()?;
        }
        Ok(())
    }
    
    fn config(&self) -> &Config {
        &self.config
    }
    
    pub fn pos(&self) -> usize {
        self.out.pos()
    }
}
