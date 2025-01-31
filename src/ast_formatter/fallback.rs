use crate::ast_formatter::AstFormatter;
use crate::error::{FormatError, FormatResult};
use crate::source_formatter::SourceFormatterSnapshot;
use std::ops::ControlFlow;

#[must_use]
pub struct Checkpoint(SourceFormatterSnapshot);

impl AstFormatter {
    fn checkpoint(&self) -> Checkpoint {
        self.constraints().fallback_stack.borrow_mut().push(());
        Checkpoint(self.out.snapshot())
    }

    /// Please return your Checkpoint here as soon as you don't need it anymore.
    pub fn close_checkpoint(&self, _: Checkpoint) {
        self.constraints().fallback_stack.borrow_mut().pop();
    }

    // todo should fallback be specific to a constraint? unless_too_wide(..).otherwise(..)
    /// Begins a fallback chain with an initial formatting attempt function
    pub fn fallback<T>(&self, first: impl FnOnce() -> FormatResult<T>) -> Fallback<T> {
        self.start_fallback().next(first)
    }

    pub fn start_fallback<T>(&self) -> Fallback<T> {
        Fallback {
            af: self,
            state: FallbackState::Continue(self.checkpoint()),
        }
    }

    pub fn fallback_with_single_line<T>(
        &self,
        first: impl FnOnce() -> FormatResult<T>,
    ) -> Fallback<T> {
        self.fallback(|| self.with_single_line(first))
    }
}

#[must_use]
pub struct Fallback<'a, T = ()> {
    af: &'a AstFormatter,
    state: FallbackState<T>,
}

type FallbackState<T> = ControlFlow<FormatResult<T>, Checkpoint>;

impl<T> Fallback<'_, T> {
    /// Chain another formatting attempt, but not the final one.
    pub fn next(mut self, attempt: impl FnOnce() -> FormatResult<T>) -> Self {
        if let FallbackState::Continue(checkpoint) = &self.state {
            match attempt() {
                // restore the checkpoint before continuing to the next formatting strategy
                Err(FormatError::Constraint(_)) => self.af.out.restore(&checkpoint.0),
                // if Ok or an unrecoverable error, we're finished
                result => self.finish(result),
            }
        }
        self
    }

    /// Chain a formatting attempt with explicit control over whether to break with a result
    /// or continue the fallback chain.
    pub fn next_control_flow<U>(
        mut self,
        attempt: impl FnOnce() -> ControlFlow<FormatResult<T>, U>,
    ) -> ControlFlow<FormatResult<T>, (Self, U)> {
        match self.state {
            FallbackState::Break(result) => ControlFlow::Break(result),
            FallbackState::Continue(ref checkpoint) => match attempt() {
                ControlFlow::Break(result) => {
                    self.finish(result);
                    let FallbackState::Break(result) = self.state else {
                        unreachable!();
                    };
                    ControlFlow::Break(result)
                }
                ControlFlow::Continue(value) => {
                    self.af.out.restore(&checkpoint.0);
                    ControlFlow::Continue((self, value))
                }
            },
        }
    }

    /// Provide the final formatting attempt.
    /// This is a required terminal operation.
    pub fn otherwise(self, final_attempt: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        match self.state {
            FallbackState::Break(result) => result,
            FallbackState::Continue(checkpoint) => {
                self.af.close_checkpoint(checkpoint);
                final_attempt()
            }
        }
    }

    fn finish(&mut self, result: FormatResult<T>) {
        let FallbackState::Continue(checkpoint) =
            std::mem::replace(&mut self.state, FallbackState::Break(result))
        else {
            panic!("fallback is already complete")
        };
        self.af.close_checkpoint(checkpoint);
    }
}
