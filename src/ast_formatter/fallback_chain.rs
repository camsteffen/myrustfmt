use crate::ast_formatter::AstFormatter;
use crate::constraint_writer::ConstraintError;
use crate::source_formatter::{FormatResult, SourceFormatter, SourceFormatterSnapshot};

pub trait HasSourceFormatter<'a> {
    fn source_formatter(&mut self) -> &mut SourceFormatter<'a>;
}

impl<'a> HasSourceFormatter<'a> for AstFormatter<'a> {
    fn source_formatter(&mut self) -> &mut SourceFormatter<'a> {
        &mut self.out
    }
}

pub fn fallback_chain<'source, CTX, Finally>(
    ctx: &mut CTX,
    chain: impl FnOnce(&mut FallbackChain<CTX, Finally>),
    finally: Finally,
) -> FormatResult
where
    CTX: HasSourceFormatter<'source>,
    Finally: Fn(&mut CTX) -> FormatResult,
{
    let snapshot = ctx.source_formatter().snapshot();
    let mut builder = FallbackChain {
        ctx,
        result: None,
        snapshot,
        finally,
    };
    chain(&mut builder);
    builder
        .result
        .expect("fallback chain must be used at least once")
}

impl<'a> AstFormatter<'a> {
    pub fn fallback_chain<'b, F: Fn(&mut Self) -> FormatResult>(
        &mut self,
        chain: impl FnOnce(&mut FallbackChain<Self, F>),
        finally: F,
    ) -> FormatResult {
        fallback_chain(self, chain, finally)
    }
}

pub struct FallbackChain<'ctx, CTX, Finally> {
    ctx: &'ctx mut CTX,
    result: Option<FormatResult>,
    snapshot: SourceFormatterSnapshot,
    finally: Finally,
}

impl<'ctx, 'source, CTX, Finally> FallbackChain<'ctx, CTX, Finally>
where
    CTX: HasSourceFormatter<'source>,
    Finally: Fn(&mut CTX) -> FormatResult,
{
    pub fn next(&mut self, f: impl FnOnce(&mut CTX) -> FormatResult) {
        if matches!(self.result, Some(Ok(_))) {
            return;
        }
        let result = f(self.ctx).and_then(|()| (self.finally)(self.ctx));
        if let Err(e) = result {
            // future-proof: only recover from constraint errors
            let _: ConstraintError = e.kind;

            self.ctx.source_formatter().restore(&self.snapshot);
        }
        self.result = Some(result);
    }
}
