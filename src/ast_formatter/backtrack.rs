use crate::ast_formatter::AstFormatter;
use crate::error::{FormatErrorKind, FormatResult};
use crate::source_formatter::checkpoint::Checkpoint;
use std::cell::Cell;

// todo can we use this more?
#[derive(Debug, Default)]
pub struct BacktrackCtxt {
    pub can_recover: Cell<bool>,
}

impl BacktrackCtxt {
    pub fn mark_can_recover(&self) {
        self.can_recover.set(true);
    }
}

impl AstFormatter {
    pub fn backtrack<'a, T>(&self) -> Backtrack<'_, 'a, T> {
        Backtrack {
            af: self,
            strategies: Vec::with_capacity(2),
        }
    }
}

pub struct Backtrack<'a, 's, T> {
    af: &'a AstFormatter,
    strategies: Vec<Box<dyn FnOnce(&BacktrackCtxt) -> FormatResult<T> + 's>>,
}

impl<'s, T> Backtrack<'_, 's, T> {
    pub fn next(mut self, strategy: impl FnOnce(&BacktrackCtxt) -> FormatResult<T> + 's) -> Self {
        self.strategies.push(Box::new(strategy));
        self
    }

    pub fn next_if(
        mut self,
        condition: bool,
        strategy: impl FnOnce(&BacktrackCtxt) -> FormatResult<T> + 's,
    ) -> Self {
        if condition {
            self.strategies.push(Box::new(strategy));
        }
        self
    }

    pub fn next_opt(
        mut self,
        strategy: Option<impl FnOnce(&BacktrackCtxt) -> FormatResult<T> + 's>,
    ) -> Self {
        if let Some(strategy) = strategy {
            self.strategies.push(Box::new(strategy));
        }
        self
    }

    pub fn result(self) -> FormatResult<T> {
        self.result_inner(None)
    }

    pub fn result_with_checkpoint(self, checkpoint: &Checkpoint) -> FormatResult<T> {
        self.result_inner(Some(checkpoint))
    }

    pub fn result_opt_checkpoint(self, checkpoint: Option<&Checkpoint>) -> FormatResult<T> {
        self.result_inner(checkpoint)
    }

    fn result_inner(self, checkpoint: Option<&Checkpoint>) -> FormatResult<T> {
        let mut iter = self.strategies.into_iter();
        let first = iter.next().expect("must provide at least one strategy");
        if iter.len() == 0 {
            // avoid creating a checkpoint if there is only one strategy
            return first(&BacktrackCtxt::default());
        }
        let new_checkpoint;
        let checkpoint = match checkpoint {
            Some(checkpoint) => checkpoint,
            None => {
                new_checkpoint = self.af.out.checkpoint();
                &new_checkpoint
            }
        };
        let mut bctx = BacktrackCtxt::default();
        let mut result = first(&bctx);
        for strategy in iter {
            let recovering = match &result {
                Ok(_) => false,
                Err(_) if bctx.can_recover.get() => true,
                Err(e) => match e.kind {
                    FormatErrorKind::Logical | FormatErrorKind::WidthLimitExceeded => true,
                    FormatErrorKind::Vertical(_) => !self.af.constraints().single_line.get(),
                    _ => false,
                },
            };
            if !recovering {
                break;
            }
            self.af.out.restore_checkpoint(checkpoint);
            bctx = BacktrackCtxt::default();
            result = strategy(&bctx);
        }
        result
    }
}
