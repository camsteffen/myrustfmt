use crate::ast_formatter::AstFormatter;
use crate::source_formatter::{FormatResult, SourceFormatterSnapshot};
use tracing::info;

/*
#[must_use]
pub struct FallbackChain<'a, 'b, T> {
    debug_name: &'static str,
    snapshot: SourceFormatterSnapshot,
    chain: Vec<(
        Box<dyn FnOnce(&mut AstFormatter<'a>) -> FormatResult + 'b>,
        &'static str,
    )>,
    finally: Box<dyn Fn(&mut AstFormatter<'a>) -> FormatResult<T> + 'b>,
}

impl<'a, 'b, T> FallbackChain<'a, 'b, T> {
    pub fn next(
        mut self,
        debug_name: &'static str,
        f: impl FnOnce(&mut AstFormatter<'a>) -> FormatResult + 'b,
    ) -> Self {
        self.chain.push((Box::new(f), debug_name));
        self
    }

    pub fn finally<U>(self, f: impl Fn(&mut AstFormatter<'a>) -> FormatResult<U> + 'b) -> FallbackChain<'a, 'b, U> {
        FallbackChain {
            debug_name: self.debug_name,
            snapshot: self.snapshot,
            chain: self.chain,
            finally: Box::new(f),
        }
    }

    pub fn execute(self, ast_formatter: &mut AstFormatter<'a>) -> FormatResult<T> {
        let mut final_result = None;
        for (f, debug_name) in self.chain {
            let result = f(ast_formatter).and_then(|()| (self.finally)(ast_formatter));
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
*/

impl<'a> AstFormatter<'a> {
    pub fn fallback_chain<'b, F: Fn(&mut Self) -> FormatResult>(
        &mut self,
        chain: impl FnOnce(&mut FallbackChain<'a, '_, F>),
        finally: F,
    ) -> FormatResult {
        let snapshot = self.out.snapshot();
        let mut builder = FallbackChain {
            ast_formatter: self,
            result: None,
            snapshot,
            finally,
        };
        chain(&mut builder);
        builder
            .result
            .expect("fallback chain must be used at least once")
    }
}

pub struct FallbackChain<'a, 'b, F> {
    ast_formatter: &'b mut AstFormatter<'a>,
    result: Option<FormatResult>,
    snapshot: SourceFormatterSnapshot,
    finally: F,
}

impl<'a, F: Fn(&mut AstFormatter<'a>) -> FormatResult> FallbackChain<'a, '_, F> {
    pub fn next(&mut self, f: impl FnOnce(&mut AstFormatter<'a>) -> FormatResult) {
        if matches!(self.result, Some(Ok(_))) {
            return;
        }
        let result = f(self.ast_formatter).and_then(|()| (self.finally)(self.ast_formatter));
        if result.is_err() {
            self.ast_formatter.out.restore(&self.snapshot);
        }
        self.result = Some(result);
    }
}
