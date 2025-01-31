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

    /// It is important to close the checkpoint *before* the final formatting strategy, since the
    /// absence of an active checkpoint indicates that constraint errors are un-recoverable.
    pub fn close_checkpoint(&self, _: Checkpoint) {
        self.constraints().fallback_stack.borrow_mut().pop();
    }

    // todo should fallback be specific to a constraint? unless_too_wide(..).otherwise(..)
    /// Begins a fallback chain with an initial formatting attempt function
    pub fn fallback<T>(&self, first: impl FnOnce() -> FormatResult<T>) -> BacktrackChain<T> {
        self.start_fallback().next(first)
    }

    pub fn start_fallback<T>(&self) -> BacktrackChain<T> {
        BacktrackChain {
            af: self,
            state: BacktrackChainState::Continue(self.checkpoint()),
        }
    }

    pub fn fallback_with_single_line<T>(
        &self,
        first: impl FnOnce() -> FormatResult<T>,
    ) -> BacktrackChain<T> {
        self.fallback(|| self.with_single_line(first))
    }
}

/// Executes a series of formatting strategies to find one that succeeds with the given constraints.
/// Each strategy is chained by calling `next` with a formatting function. If a strategy fails with
/// a constraint error, it will restore a checkpoint and run the next strategy. If a strategy
/// succeeds, it will hold the result until the end, and all subsequent strategies will be ignored.
#[must_use]
pub struct BacktrackChain<'a, T = ()> {
    af: &'a AstFormatter,
    state: BacktrackChainState<T>,
}

type BacktrackChainState<T> = ControlFlow<FormatResult<T>, Checkpoint>;

impl<T> BacktrackChain<'_, T> {
    /// Chain another formatting attempt, but not the final one.
    pub fn next(mut self, attempt: impl FnOnce() -> FormatResult<T>) -> Self {
        if let BacktrackChainState::Continue(checkpoint) = &self.state {
            match attempt() {
                // restore the checkpoint before continuing to the next formatting strategy
                Err(FormatError::Constraint(_)) => self.af.out.restore(&checkpoint.0),
                // if Ok or an unrecoverable error, we're finished
                result => match std::mem::replace(&mut self.state, BacktrackChainState::Break(result)) {
                    BacktrackChainState::Continue(checkpoint) => self.af.close_checkpoint(checkpoint),
                    _ => unreachable!(),
                },
            }
        }
        self
    }

    /// Chain a formatting attempt with explicit control over whether to break with a result
    /// or continue the fallback chain.
    pub fn next_control_flow<U>(
        self,
        attempt: impl FnOnce() -> ControlFlow<FormatResult<T>, U>,
    ) -> ControlFlow<FormatResult<T>, (Self, U)> {
        match self.state {
            BacktrackChainState::Break(result) => ControlFlow::Break(result),
            BacktrackChainState::Continue(ref checkpoint) => match attempt() {
                ControlFlow::Break(result) => {
                    let BacktrackChainState::Continue(checkpoint) = self.state else {
                        unreachable!();
                    };
                    self.af.close_checkpoint(checkpoint);
                    ControlFlow::Break(result)
                }
                ControlFlow::Continue(value) => {
                    self.af.out.restore(&checkpoint.0);
                    ControlFlow::Continue((self, value))
                }
            },
        }
    }

    /// Provide the final formatting attempt. This is a required terminal operation.
    pub fn otherwise(self, final_attempt: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        match self.state {
            BacktrackChainState::Break(result) => result,
            BacktrackChainState::Continue(checkpoint) => {
                self.af.close_checkpoint(checkpoint);
                final_attempt()
            }
        }
    }
}
