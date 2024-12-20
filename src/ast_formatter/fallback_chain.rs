use crate::ast_formatter::AstFormatter;
use crate::error::{FormatErrorKind, FormatResult};
use crate::source_formatter::{SourceFormatter, SourceFormatterSnapshot};

pub trait ResultFallback<T> {
    fn fallback(
        self,
        af: &AstFormatter,
        snapshot: &SourceFormatterSnapshot,
        fallback: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T>;
}

impl<T> ResultFallback<T> for FormatResult<T> {
    fn fallback(
        self,
        af: &AstFormatter,
        snapshot: &SourceFormatterSnapshot,
        fallback: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let should_fallback = match &self {
            Ok(_) => false,
            Err(e) => match e.kind {
                FormatErrorKind::Parse(_) => false,
                FormatErrorKind::Constraint(_) => true,
            },
        };
        if !should_fallback {
            return self;
        }
        af.out.restore(snapshot);
        fallback()
    }
}

impl AstFormatter {
    fn fallback<T>(&self, first: impl FnOnce() -> FormatResult<T>) -> FallbackResult<T> {
        let snapshot = self.out.snapshot();
        let result = first();
        FallbackResult { snapshot, result }
    }
}

struct FallbackResult<T> {
    snapshot: SourceFormatterSnapshot,
    result: FormatResult<T>,
}

impl<T> FallbackResult<T> {
    fn next(mut self, af: &AstFormatter, fallback: impl FnOnce() -> FormatResult<T>) -> Self {
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
}
