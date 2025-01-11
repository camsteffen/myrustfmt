use crate::ast_formatter::AstFormatter;
use crate::error::{FormatError, FormatResult};
use crate::source_formatter::{SourceFormatter, SourceFormatterSnapshot};

impl AstFormatter {
    pub fn fallback<T>(&self, first: impl FnOnce() -> FormatResult<T>) -> Fallback<T> {
        let out = &self.out;
        let snapshot = out.snapshot();
        let result = first();
        Fallback {
            out,
            snapshot,
            result,
        }
    }
}

#[must_use]
pub struct Fallback<'a, T = ()> {
    out: &'a SourceFormatter,
    snapshot: SourceFormatterSnapshot,
    result: FormatResult<T>,
}

impl<T> Fallback<'_, T> {
    pub fn next(mut self, fallback: impl FnOnce() -> FormatResult<T>) -> Self {
        if self.is_done() {
            return self;
        }
        self.out.restore(&self.snapshot);
        self.result = fallback();
        self
    }

    /// Returns true if the result is either Ok or a non-recoverable error
    fn is_done(&self) -> bool {
        match self.result {
            Ok(_) | Err(FormatError::Parse(_)) => true,
            Err(FormatError::Constraint(_)) => false,
        }
    }

    pub fn result(self) -> FormatResult<T> {
        self.result
    }

    pub fn peek_result(&self) -> &FormatResult<T> {
        &self.result
    }

    pub fn snapshot(&self) -> &SourceFormatterSnapshot {
        &self.snapshot
    }
}
