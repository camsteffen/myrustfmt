use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::checkpoint::Checkpoint;
use crate::error::{FormatError, FormatResult};

impl AstFormatter {
    // todo should backtrack be specific to a constraint? unless_too_wide(..).otherwise(..)
    /// See [`Backtrack`]
    pub fn backtrack<T>(&self) -> Backtrack<T> {
        Backtrack {
            af: self,
            state: BacktrackState::Init,
        }
    }

    pub fn backtrack_from_checkpoint<T>(&self, checkpoint: Checkpoint) -> Backtrack<T> {
        Backtrack {
            af: self,
            state: BacktrackState::Incomplete(checkpoint),
        }
    }
}

/// Executes a series of formatting strategies to find one that succeeds with the given constraints.
///
/// Each strategy is chained by calling `next` with a formatting function, and the final strategy
/// is chained with `otherwise`. If a strategy fails with a constraint error, it will restore a
/// checkpoint before running the next strategy. If a strategy succeeds, it will hold the result
/// until the end, and all subsequent strategies will be ignored.
///
/// Backtrack is a higher abstraction than using Checkpoint directly, and should be preferred for
/// simple cases since it ensures that the Checkpoint is dropped at the right time.
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
                // N.B. this drops the checkpoint
                result => self.state = BacktrackState::Done(result),
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

    /// Provides the final formatting strategy and returns the result of the backtracking chain.
    /// This is a required terminal operation.
    pub fn otherwise(self, strategy: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        match self.state {
            BacktrackState::Init => strategy(),
            BacktrackState::Incomplete(checkpoint) => {
                // N.B. decrement the checkpoint counter _before_ the final strategy
                drop(checkpoint);
                strategy()
            }
            BacktrackState::Done(result) => result,
        }
    }
}
