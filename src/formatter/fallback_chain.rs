use tracing::info;
use crate::formatter::{FormatResult, Formatter, FormatterSnapshot};

#[must_use]
pub struct FallbackChain<'a, 'b> {
    debug_name: &'static str,
    out: &'b mut Formatter<'a>,
    snapshot: FormatterSnapshot,
    result: Option<FormatResult>,
}

impl<'a> FallbackChain<'a, '_> {
    pub fn next(
        mut self,
        debug_name: &'static str,
        f: impl FnOnce(&mut Formatter<'a>) -> FormatResult,
    ) -> Self {
        if matches!(self.result, None | Some(Err(_))) {
            let result = f(self.out);
            match result {
                Ok(_) => info!("{}: {} succeeded", self.debug_name, debug_name),
                Err(e) => info!("{}: {} failed: {e:?}", self.debug_name, debug_name),
            }
            if let Err(_) = result {
                self.out.restore(&self.snapshot);
            }
            self.result = Some(result);
        }
        self
    }

    pub fn result(self) -> FormatResult {
        self.result.expect("fallback chain cannot be empty")
    }
}

impl<'a> Formatter<'a> {
    pub fn fallback_chain(&mut self, debug_name: &'static str) -> FallbackChain<'a, '_> {
        let snapshot = self.snapshot();
        FallbackChain {
            debug_name,
            out: self,
            snapshot,
            result: None,
        }
    }
}
