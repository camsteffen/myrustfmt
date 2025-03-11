use crate::ast_formatter::AstFormatter;
use crate::constraint_writer::{ConstraintRecoveryMode, MAX_CONSTRAINT_RECOVERY_MODE};
use crate::error::{FormatError, FormatResult};
use crate::source_formatter::SourceFormatterCheckpoint;

impl AstFormatter {
    // todo should backtrack be specific to a constraint? unless_too_wide(..).otherwise(..)
    /// See [`Backtrack`]
    pub fn backtrack<T>(&self) -> Backtrack<T> {
        Backtrack {
            af: self,
            state: BacktrackState::Init,
        }
    }

    pub fn backtrack_from_checkpoint<T>(&self, checkpoint: SourceFormatterCheckpoint) -> Backtrack<T> {
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
    Incomplete(SourceFormatterCheckpoint),
    Done(FormatResult<T>),
}

impl<T> Backtrack<'_, T> {
    /// Provides the next formatting strategy, but not the final one.
    pub fn next(mut self, strategy: impl FnOnce() -> FormatResult<T>) -> Self {
        self.do_next(MAX_CONSTRAINT_RECOVERY_MODE, strategy);
        self
    }
    
    pub fn next_with_constraint_recovery_mode(mut self, mode: ConstraintRecoveryMode, strategy: impl FnOnce() -> FormatResult<T>) -> Self {
        self.do_next(mode, strategy);
        self
    }
    
    fn do_next(&mut self, mode: ConstraintRecoveryMode, strategy: impl FnOnce() -> FormatResult<T>) {
        if let BacktrackState::Init = self.state {
            self.state = BacktrackState::Incomplete(self.af.out.checkpoint());
        }
        if let BacktrackState::Incomplete(checkpoint) = &self.state {
            let result = {
                let _guard = self.af.out.constraint_recovery_mode_max(mode);
                strategy()
            };
            match result {
                // restore the checkpoint before continuing to the next formatting strategy
                Err(FormatError::Constraint(_)) => self.af.out.restore_checkpoint(checkpoint),
                // if Ok or an unrecoverable error, we're finished
                result => self.state = BacktrackState::Done(result),
            }
        }
    }

    pub fn next_if(mut self, condition: bool, strategy: impl Fn() -> FormatResult<T>) -> Self {
        if condition {
            self.do_next(MAX_CONSTRAINT_RECOVERY_MODE, strategy);
        }
        self
    }

    pub fn next_opt(mut self, strategy: Option<impl Fn() -> FormatResult<T>>) -> Self {
        if let Some(strategy) = strategy {
            self.do_next(MAX_CONSTRAINT_RECOVERY_MODE, strategy);
        }
        self
    }

    /// Provides the final formatting strategy and returns the result of the backtracking chain.
    /// This is a required terminal operation.
    pub fn otherwise(self, strategy: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        match self.state {
            BacktrackState::Init | BacktrackState::Incomplete(_) => strategy(),
            BacktrackState::Done(result) => result,
        }
    }
}
