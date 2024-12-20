use crate::ast_formatter::AstFormatter;
use crate::error::{FormatErrorKind, FormatResult};
use crate::source_formatter::SourceFormatterSnapshot;

impl AstFormatter {
    pub fn fallback<T>(&self, first: impl FnOnce() -> FormatResult<T>) -> FallbackResult<T> {
        let snapshot = self.out.snapshot();
        let result = first();
        FallbackResult { snapshot, result }
    }
}

#[must_use]
pub struct FallbackResult<T> {
    snapshot: SourceFormatterSnapshot,
    result: FormatResult<T>,
}

impl<T> FallbackResult<T> {
    pub fn next(mut self, af: &AstFormatter, fallback: impl FnOnce() -> FormatResult<T>) -> Self {
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
        af.out.restore(&self.snapshot);
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
