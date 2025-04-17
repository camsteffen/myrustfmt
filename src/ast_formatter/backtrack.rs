use crate::ast_formatter::AstFormatter;
use crate::constraint_writer::ConstraintRecoveryMode;
use crate::error::{ConstraintErrorKind, FormatResult};
use crate::source_formatter::checkpoint::Checkpoint;

impl AstFormatter {
    // todo should backtrack be specific to a constraint? unless_too_wide(..).otherwise(..)
    /// See [`Backtrack`]
    pub fn backtrack<T>(&self) -> Backtrack<T> {
        Backtrack {
            af: self,
            state: BacktrackState::Init,
        }
    }

    pub fn backtrack_from_checkpoint<'a, T>(
        &'a self,
        checkpoint: Checkpoint<'a>,
    ) -> Backtrack<'a, T> {
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
    state: BacktrackState<'a, T>,
}

#[derive(Default)]
enum BacktrackState<'a, T> {
    #[default]
    Init,
    Incomplete(Checkpoint<'a>),
    Done(Checkpoint<'a>, FormatResult<T>),
}

impl<T> Backtrack<'_, T> {
    /// Provides the next formatting strategy, but not the final one.
    pub fn next(mut self, strategy: impl FnOnce() -> FormatResult<T>) -> Self {
        self.do_next(self.af.out.max_recovery_mode(), strategy);
        self
    }

    pub fn next_with_constraint_recovery_mode(
        mut self,
        mode: ConstraintRecoveryMode,
        strategy: impl FnOnce() -> FormatResult<T>,
    ) -> Self {
        self.do_next(mode, strategy);
        self
    }

    fn do_next(
        &mut self,
        mode: ConstraintRecoveryMode,
        strategy: impl FnOnce() -> FormatResult<T>,
    ) {
        match self.state {
            BacktrackState::Init => {
                self.state = BacktrackState::Incomplete(self.af.out.checkpoint());
            }
            BacktrackState::Incomplete(ref checkpoint) => {
                self.af.out.restore_checkpoint(checkpoint);
            }
            BacktrackState::Done(..) => return,
        }
        let result = self.af.out.with_constraint_recovery_mode_max(mode, strategy);
        match result {
            Err(e) if e.kind == ConstraintErrorKind::NextStrategy => {}
            _ => {
                let BacktrackState::Incomplete(checkpoint) = std::mem::take(&mut self.state) else {
                    unreachable!()
                };
                self.state = BacktrackState::Done(checkpoint, result);
            }
        }
        // todo this is an experiment
        self.filter_err_is(ConstraintErrorKind::WidthLimitExceeded);
    }

    pub fn next_if(mut self, condition: bool, strategy: impl Fn() -> FormatResult<T>) -> Self {
        if condition {
            self.do_next(self.af.out.max_recovery_mode(), strategy);
        }
        self
    }

    pub fn next_opt(mut self, strategy: Option<impl Fn() -> FormatResult<T>>) -> Self {
        if let Some(strategy) = strategy {
            self.do_next(self.af.out.max_recovery_mode(), strategy);
        }
        self
    }

    /// Provides the final formatting strategy and returns the result of the backtracking chain.
    /// This is a required terminal operation.
    pub fn otherwise(self, strategy: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        match self.state {
            BacktrackState::Init => strategy(),
            BacktrackState::Incomplete(checkpoint) => {
                self.af.out.restore_checkpoint(&checkpoint);
                strategy()
            }
            BacktrackState::Done(_, result) => result,
        }
    }

    // todo handle bad block comments here?
    pub fn unless_multi_line(mut self) -> Self {
        self.filter_err_is(ConstraintErrorKind::NewlineNotAllowed);
        self
    }

    pub fn unless_too_wide(mut self) -> Self {
        self.filter_err_is(ConstraintErrorKind::WidthLimitExceeded);
        self
    }

    fn filter_err_is(&mut self, kind: ConstraintErrorKind) {
        self.state = match std::mem::take(&mut self.state) {
            BacktrackState::Done(checkpoint, Err(e)) if e.kind == kind => {
                BacktrackState::Incomplete(checkpoint)
            }
            state => state,
        };
    }
}
