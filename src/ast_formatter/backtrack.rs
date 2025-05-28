use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;
use crate::source_formatter::checkpoint::Checkpoint;

impl AstFormatter {
    /// See [`Backtrack`]
    pub fn backtrack<T>(&self) -> Backtrack<T> {
        Backtrack {
            af: self,
            state: None,
        }
    }

    pub fn backtrack_from_checkpoint<'a, T>(
        &'a self,
        checkpoint: Checkpoint<'a>,
    ) -> Backtrack<'a, T> {
        Backtrack {
            af: self,
            state: Some(BacktrackState {
                checkpoint,
                last_result: None,
            }),
        }
    }
}

/// Executes a series of formatting strategies to find one that succeeds with the given constraints.
///
/// Each strategy is chained by calling `next` with a formatting function, and the final strategy
/// is chained with `otherwise`. If a strategy fails with a constraint error, it will restore a
/// checkpoint before running the next strategy. If a strategy succeeds, it will hold the result
/// until the end, and all subsequent strategies will be ignored.
#[must_use]
pub struct Backtrack<'a, T = ()> {
    af: &'a AstFormatter,
    state: Option<BacktrackState<'a, T>>,
}

struct BacktrackState<'a, T> {
    checkpoint: Checkpoint<'a>,
    last_result: Option<FormatResult<T>>,
}

impl<T> Backtrack<'_, T> {
    pub fn next(mut self, strategy: impl FnOnce() -> FormatResult<T>) -> Self {
        self.do_next(strategy);
        self
    }

    pub fn next_if(mut self, condition: bool, strategy: impl Fn() -> FormatResult<T>) -> Self {
        if condition {
            self.do_next(strategy);
        }
        self
    }

    pub fn next_opt(mut self, strategy: Option<impl FnOnce() -> FormatResult<T>>) -> Self {
        if let Some(strategy) = strategy {
            self.do_next(strategy);
        }
        self
    }

    pub fn result(self) -> FormatResult<T> {
        self.state.and_then(|state| state.last_result).unwrap()
    }

    fn do_next(&mut self, strategy: impl FnOnce() -> FormatResult<T>) {
        let state = match &mut self.state {
            None => {
                let checkpoint = self.af.out.checkpoint();
                self.state.insert(BacktrackState {
                    checkpoint,
                    last_result: None,
                })
            }
            Some(state) => {
                if matches!(state.last_result, Some(Ok(_))) {
                    return;
                }
                self.af.out.restore_checkpoint(&state.checkpoint);
                state
            }
        };
        let result = strategy();
        state.last_result = Some(result);
    }
}
