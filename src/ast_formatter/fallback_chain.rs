use crate::ast_formatter::AstFormatter;
use crate::constraint_writer::ConstraintError;
use crate::source_formatter::{FormatResult, SourceFormatterSnapshot};
use tracing::info;

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
        if let Err(e) = result {
            // future-proof: only recover from constraint errors
            let _: ConstraintError = e.kind;

            self.ast_formatter.out.restore(&self.snapshot);
        }
        self.result = Some(result);
    }
}
