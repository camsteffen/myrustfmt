use crate::ast_formatter::AstFormatter;
use crate::error::{FormatError, FormatResult};
use crate::source_formatter::SourceFormatterSnapshot;

impl AstFormatter {
    // todo should fallback be specific to a constraint? unless_too_wide(..).otherwise(..)
    /// Begins a fallback chain with an initial formatting attempt function
    pub fn fallback<T>(&self, first: impl FnOnce() -> FormatResult<T>) -> Fallback<T> {
        let has_fallback_prev = self.constraints().has_fallback.replace(true);
        let snapshot = Box::new(self.out.snapshot());
        let state = FallbackState::Incomplete {
            has_fallback_prev,
            snapshot,
        };
        let mut fallback = Fallback { af: self, state };
        let result = first();
        fallback.maybe_complete(result);
        fallback
    }
}

#[must_use]
pub struct Fallback<'a, T = ()> {
    af: &'a AstFormatter,
    state: FallbackState<T>,
}

enum FallbackState<T> {
    Complete(FormatResult<T>),
    Incomplete {
        has_fallback_prev: bool,
        snapshot: Box<SourceFormatterSnapshot>,
    },
}

impl<T> Fallback<'_, T> {
    /// Chain another formatting attempt, but not the final one.
    pub fn next(mut self, fallback: impl FnOnce() -> FormatResult<T>) -> Self {
        match &self.state {
            FallbackState::Complete(_) => self,
            FallbackState::Incomplete { snapshot, .. } => {
                self.af.out.restore(snapshot);
                let result = fallback();
                self.maybe_complete(result);
                self
            }
        }
    }

    /// Provide the final formatting attempt.
    /// This is a required terminal operation.
    pub fn otherwise(self, fallback: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        match self.state {
            FallbackState::Complete(result) => result,
            FallbackState::Incomplete {
                has_fallback_prev,
                snapshot,
            } => {
                self.af.out.restore(&snapshot);
                self.af.constraints().has_fallback.set(has_fallback_prev);
                fallback()
            }
        }
    }

    fn maybe_complete(&mut self, result: FormatResult<T>) {
        match self.state {
            FallbackState::Complete(_) => panic!("fallback is already complete"),
            FallbackState::Incomplete {
                has_fallback_prev, ..
            } => {
                if is_result_terminal(&result) {
                    self.af.constraints().has_fallback.set(has_fallback_prev);
                    self.state = FallbackState::Complete(result);
                }
            }
        }
    }
}

/// Returns true if the result is either Ok or a non-recoverable error
fn is_result_terminal<T>(result: &FormatResult<T>) -> bool {
    match result {
        Ok(_) | Err(FormatError::Parse(_)) => true,
        Err(FormatError::Constraint(_)) => false,
    }
}
