use crate::ast_formatter::AstFormatter;
use crate::constraint_writer::ConstraintError;
use crate::source_formatter::{FormatResult, SourceFormatter, SourceFormatterSnapshot};

pub fn fallback_chain<Finally>(
    out: &SourceFormatter,
    chain: impl FnOnce(&mut FallbackChain<'_, Finally>),
    finally: Finally,
) -> FormatResult
where
    Finally: Fn() -> FormatResult,
{
    let mut builder = FallbackChain {
        out,
        result: None,
        snapshot: out.snapshot(),
        finally,
    };
    chain(&mut builder);
    builder
        .result
        .expect("fallback chain must be used at least once")
}

impl<'a> AstFormatter {
    pub fn fallback_chain<'b, F: Fn() -> FormatResult>(
        &self,
        chain: impl FnOnce(&mut FallbackChain<F>),
        finally: F,
    ) -> FormatResult {
        fallback_chain(&self.out, chain, finally)
    }
}

pub struct FallbackChain<'a, Finally> {
    out: &'a SourceFormatter,
    result: Option<FormatResult>,
    snapshot: SourceFormatterSnapshot,
    finally: Finally,
}

impl<'a, Finally> FallbackChain<'a, Finally>
where
    Finally: Fn() -> FormatResult,
{
    pub fn next(&mut self, f: impl FnOnce() -> FormatResult) {
        if matches!(self.result, Some(Ok(_))) {
            return;
        }
        let result = f().and_then(|()| (self.finally)());
        if let Err(e) = result {
            // future-proof: only recover from constraint errors
            let _: ConstraintError = e.kind;

            self.out.restore(&self.snapshot);
        }
        self.result = Some(result);
    }
}
