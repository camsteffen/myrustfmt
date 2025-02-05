use crate::ast_formatter::AstFormatter;
use crate::ast_formatter::checkpoint::{Checkpoint, Lookahead};
use crate::error::{FormatError, FormatResult};
use std::ops::ControlFlow;

macro_rules! try_backtrack {
    ($backtrack:expr) => {{
        match $backtrack.into_inner() {
            ::std::ops::ControlFlow::Break(result) => return result,
            ::std::ops::ControlFlow::Continue(checkpoint) => checkpoint,
        }
    }};
}
pub(crate) use try_backtrack;

impl AstFormatter {
    // todo should fallback be specific to a constraint? unless_too_wide(..).otherwise(..)
    pub fn backtrack<T>(&self) -> Backtrack<T> {
        self.backtrack_from_checkpoint(self.open_checkpoint())
    }
    
    pub fn backtrack_from_checkpoint<T>(&self, checkpoint: Checkpoint) -> Backtrack<T> {
        Backtrack {
            af: self,
            state: BacktrackState::Continue(checkpoint),
        }
    }

    pub fn backtrack_with_single_line<T>(
        &self,
        first: impl FnOnce() -> FormatResult<T>,
    ) -> Backtrack<T> {
        self.backtrack().next(|| self.with_single_line(first))
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

type BacktrackState<T> = ControlFlow<FormatResult<T>, Checkpoint>;

impl<T> Backtrack<'_, T> {
    /// Provides the next formatting strategy, but not the final one.
    pub fn next(mut self, strategy: impl FnOnce() -> FormatResult<T>) -> Self {
        if let BacktrackState::Continue(checkpoint) = &self.state {
            match strategy() {
                // restore the checkpoint before continuing to the next formatting strategy
                Err(FormatError::Constraint(_)) => self.af.restore_checkpoint(checkpoint),
                // if Ok or an unrecoverable error, we're finished
                result => match std::mem::replace(&mut self.state, BacktrackState::Break(result)) {
                    BacktrackState::Continue(checkpoint) => self.af.close_checkpoint(checkpoint),
                    _ => unreachable!(),
                },
            }
        }
        self
    }

    pub fn next_with_checkpoint(mut self, strategy: impl FnOnce(&Checkpoint) -> FormatResult<T>) -> Self {
        if let BacktrackState::Continue(checkpoint) = &self.state {
            match strategy(checkpoint) {
                // restore the checkpoint before continuing to the next formatting strategy
                Err(FormatError::Constraint(_)) => self.af.restore_checkpoint(checkpoint),
                // if Ok or an unrecoverable error, we're finished
                result => match std::mem::replace(&mut self.state, BacktrackState::Break(result)) {
                    BacktrackState::Continue(checkpoint) => self.af.close_checkpoint(checkpoint),
                    _ => unreachable!(),
                },
            }
        }
        self
    }

    /// Provides the final formatting strategy and returns the result of the backtracking chain.
    /// This is a required terminal operation.
    pub fn otherwise(self, final_attempt: impl FnOnce() -> FormatResult<T>) -> FormatResult<T> {
        match self.state {
            BacktrackState::Break(result) => result,
            BacktrackState::Continue(checkpoint) => {
                self.af.close_checkpoint(checkpoint);
                final_attempt()
            }
        }
    }
    
    pub fn into_inner(self) -> BacktrackState<T> {
        self.state
    }

    /// Provides the next formatting strategy with explicit control of whether to break with a
    /// result or continue with subsequent strategies.
    pub fn next_control_flow<C>(
        self,
        strategy: impl FnOnce() -> ControlFlow<FormatResult<T>, C>,
    ) -> ControlFlow<FormatResult<T>, (Self, C)> {
        match self.state {
            BacktrackState::Break(result) => ControlFlow::Break(result),
            BacktrackState::Continue(ref checkpoint) => match strategy() {
                ControlFlow::Break(result) => {
                    let BacktrackState::Continue(checkpoint) = self.state else {
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
        }
    }

    pub fn next_control_flow_lookahead<C>(
        self,
        strategy: impl FnOnce() -> ControlFlow<FormatResult<T>, C>,
    ) -> ControlFlow<FormatResult<T>, (Self, Lookahead, C)> {
        match self.state {
            BacktrackState::Break(result) => ControlFlow::Break(result),
            BacktrackState::Continue(ref checkpoint) => match strategy() {
                ControlFlow::Break(value) => {
                    let BacktrackState::Continue(checkpoint) = self.state else {
                        unreachable!();
                    };
                    self.af.close_checkpoint(checkpoint);
                    ControlFlow::Break(value) 
                }
                ControlFlow::Continue(value) => {
                    let lookahead = self.af.capture_lookahead(checkpoint);
                    ControlFlow::Continue((self, lookahead, value))
                }
            },
        }
    }
}

/*
impl AstFormatter {
    pub fn choose_fewer_lines(
        &self,
        checkpoint: &Checkpoint,
        strategy_a: impl FnOnce() -> FormatResult,
        strategy_b: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        let first_line = self.out.line();
        let result_a = self.lookahead(checkpoint, ||{
            strategy_a()?;
            Ok(self.out.line() - first_line)
        });
        match result_a {
            Err(FormatError::Constraint(_)) => strategy_b(),
            Err(e) => Err(e),
            Ok((lookahead_a, lines_a)) => match strategy_b() {
                Err(FormatError::Constraint(_)) => {
                    self.restore_checkpoint(checkpoint);
                    self.restore_lookahead(lookahead_a);
                    Ok(())
                }
                Err(e) => Err(e),
                Ok(()) =>  {
                    let lines_b = self.out.line() - first_line;
                    if lines_b < lines_a {
                        return Ok(())
                    }
                    self.restore_checkpoint(checkpoint);
                    self.restore_lookahead(lookahead_a);
                    Ok(())
                }
            }
        }
    }

    // always restores the checkpoint
    fn lookahead<T>(
        &self,
        checkpoint: &Checkpoint,
        format: impl Fn() -> FormatResult<T>,
    ) -> FormatResult<(Lookahead, T)> {
        let buffer = self.out.take_buffer();
        self.out.write_some_whitespace_to_restore_line_line();
        match format() {
            Err(e) => {
                self.restore_checkpoint(checkpoint);
                Err(e)
            }
            Ok(value) => {
                let lookahead = self.out.split_off_out(checkpoint.)
                self.restore_checkpoint(checkpoint);
                Ok((lookahead, value))
            }
        }
    }

}

type Lookahead = String;


 */
