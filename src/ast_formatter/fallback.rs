use crate::ast_formatter::AstFormatter;
use crate::error::{FormatErrorKind, FormatResult};
use crate::source_formatter::{SourceFormatter, SourceFormatterSnapshot};

impl AstFormatter {
    pub fn fallback<T>(&self, first: impl FnOnce() -> FormatResult<T>) -> FallbackResult<T> {
        let out = &self.out;
        let snapshot = out.snapshot();
        let result = first();
        FallbackResult { out, snapshot, result }
    }
}

#[must_use]
pub struct FallbackResult<'a, T> {
    out: &'a SourceFormatter,
    snapshot: SourceFormatterSnapshot,
    result: FormatResult<T>,
}

impl<T> FallbackResult<'_, T> {
    pub fn next(mut self, fallback: impl FnOnce() -> FormatResult<T>) -> Self {
        let should_fallback = match &self.result {
            Ok(_) => false,
            Err(e) => match e.kind {
                FormatErrorKind::Parse(_) => false,
                FormatErrorKind::Constraint(_) => true,
            },
        };
        if !should_fallback {
            return self;
        }
        self.out.restore(&self.snapshot);
        self.result = fallback();
        self
    }

    pub fn result(self) -> FormatResult<T> {
        self.result
    }

    pub fn result_ref(&self) -> &FormatResult<T> {
        &self.result
    }
}
