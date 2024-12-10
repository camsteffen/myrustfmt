use crate::ast_formatter::AstFormatter;
use crate::source_formatter::{FormatResult, SourceFormatterSnapshot};
use tracing::info;

#[must_use]
pub struct FallbackChain<'a, 'b> {
    debug_name: &'static str,
    snapshot: SourceFormatterSnapshot,
    chain: Vec<(
        Box<dyn FnOnce(&mut AstFormatter<'a>) -> FormatResult + 'b>,
        &'static str,
    )>,
    finally: Option<Box<dyn Fn(&mut AstFormatter<'a>) -> FormatResult + 'b>>,
}

impl<'a, 'b> FallbackChain<'a, 'b> {
    pub fn next(
        mut self,
        debug_name: &'static str,
        f: impl FnOnce(&mut AstFormatter<'a>) -> FormatResult + 'b,
    ) -> Self {
        self.chain.push((Box::new(f), debug_name));
        self
    }

    pub fn finally(mut self, f: impl Fn(&mut AstFormatter<'a>) -> FormatResult + 'b) -> Self {
        self.finally = Some(Box::new(f));
        self
    }

    pub fn execute(self, ast_formatter: &mut AstFormatter<'a>) -> FormatResult {
        let mut final_result = None;
        for (f, debug_name) in self.chain {
            let mut result = f(ast_formatter);
            if let Some(finally) = &self.finally {
                result = result.and_then(|()| finally(ast_formatter));
            }
            match final_result.insert(result) {
                Ok(_) => {
                    info!("{}: {} succeeded", self.debug_name, debug_name);
                    break;
                }
                Err(e) => {
                    info!("{}: {} failed: {e:?}", self.debug_name, debug_name);
                    ast_formatter.out.restore(&self.snapshot);
                }
            }
        }
        final_result.expect("fallback chain cannot be empty")
    }
}

impl<'a> AstFormatter<'a> {
    pub fn fallback_chain<'b>(&mut self, debug_name: &'static str) -> FallbackChain<'a, 'b> {
        FallbackChain {
            debug_name,
            snapshot: self.out.snapshot(),
            chain: Vec::new(),
            finally: None,
        }
    }
}
