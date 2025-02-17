use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::checkpoint::Checkpoint;
use crate::error::{FormatError, FormatResult};
use std::ops::ControlFlow;

impl AstFormatter {
    // todo should backtrack be specific to a constraint? unless_too_wide(..).otherwise(..)
    pub fn backtrack<T>(&self) -> Backtrack<T> {
        Backtrack {
            af: &self,
            state: BacktrackState::Init,
        }
    }
}

/// Executes a series of formatting strategies to find one that succeeds with the given constraints.
/// Each strategy is chained by calling `next` with a formatting function, and the final strategy
/// is chained with `otherwise`. If a strategy fails with a constraint error, it will restore a
/// checkpoint before running the next strategy. If a strategy succeeds, it will hold the result
/// until the end, and all subsequent strategies will be ignored.
#[must_use]
pub struct Backtrack<'a, T = ()> {
    af: &'a AstFormatter,
    state: BacktrackState<T>,
}

enum BacktrackState<T> {
    Init,
    Incomplete(Checkpoint),
    Done(FormatResult<T>),
}

impl<T> Backtrack<'_, T> {
    /// Provides the next formatting strategy, but not the final one.
    pub fn next(mut self, strategy: impl FnOnce() -> FormatResult<T>) -> Self {
        if let BacktrackState::Init = self.state {
            self.state = BacktrackState::Incomplete(self.af.open_checkpoint());
        }
        if let BacktrackState::Incomplete(checkpoint) = &self.state {
            match strategy() {
                // restore the checkpoint before continuing to the next formatting strategy
                Err(FormatError::Constraint(_)) => self.af.restore_checkpoint(checkpoint),
                // if Ok or an unrecoverable error, we're finished
                result => match std::mem::replace(&mut self.state, BacktrackState::Done(result)) {
                    BacktrackState::Incomplete(checkpoint) => self.af.close_checkpoint(checkpoint),
                    _ => unreachable!(),
                },
            }
        }
        self
    }

    pub fn next_if(self, condition: bool, strategy: impl Fn() -> FormatResult<T>) -> Self {
        if condition { self.next(strategy) } else { self }
    }

    pub fn next_opt(self, strategy: Option<impl Fn() -> FormatResult<T>>) -> Self {
        match strategy {
            None => self,
            Some(strategy) => self.next(strategy),
        }
    }

    pub fn next_single_line(self, strategy: impl FnOnce() -> FormatResult<T>) -> Self {
        let af = self.af;
        self.next(|| af.with_single_line(strategy))
    }

    /// Provides the final formatting strategy and returns the result of the backtracking chain.
    /// This is a required terminal operation.
    pub fn otherwise(self, final_attempt: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        match self.state {
            BacktrackState::Init => final_attempt(),
            BacktrackState::Incomplete(checkpoint) => {
                self.af.close_checkpoint(checkpoint);
                final_attempt()
            }
            BacktrackState::Done(result) => result,
        }
    }

    /// Provides the next formatting strategy with explicit control of whether to break with a
    /// result or continue with subsequent strategies.
    pub fn next_control_flow<C>(
        mut self,
        strategy: impl FnOnce() -> ControlFlow<FormatResult<T>, C>,
    ) -> ControlFlow<FormatResult<T>, (Self, C)> {
        if let BacktrackState::Init = self.state {
            self.state = BacktrackState::Incomplete(self.af.open_checkpoint());
        }
        match self.state {
            BacktrackState::Init => unreachable!(),
            BacktrackState::Incomplete(ref checkpoint) => match strategy() {
                ControlFlow::Break(result) => {
                    let BacktrackState::Incomplete(checkpoint) = self.state else {
                        unreachable!();
                    };
                    self.af.close_checkpoint(checkpoint);
                    ControlFlow::Break(result)
                }
                ControlFlow::Continue(value) => {
                    self.af.restore_checkpoint(checkpoint);
                    ControlFlow::Continue((self, value))
                }
            },
            BacktrackState::Done(result) => ControlFlow::Break(result),
        }
    }
}
