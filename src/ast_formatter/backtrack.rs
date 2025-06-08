use crate::ast_formatter::AstFormatter;
use crate::error::{FormatErrorKind, FormatResult};
use crate::source_formatter::checkpoint::Checkpoint;

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
    strategies: Vec<Box<dyn FnOnce() -> FormatResult<T> + 's>>,
}

impl<'s, T> Backtrack<'_, 's, T> {
    pub fn next(mut self, strategy: impl FnOnce() -> FormatResult<T> + 's) -> Self {
        self.strategies.push(Box::new(strategy));
        self
    }

    pub fn next_if(
        mut self,
        condition: bool,
        strategy: impl FnOnce() -> FormatResult<T> + 's,
    ) -> Self {
        if condition {
            self.strategies.push(Box::new(strategy));
        }
        self
    }

    pub fn next_opt(mut self, strategy: Option<impl FnOnce() -> FormatResult<T> + 's>) -> Self {
        if let Some(strategy) = strategy {
            self.strategies.push(Box::new(strategy));
        }
        self
    }

    pub fn result(self) -> FormatResult<T> {
        self.result_inner(None)
    }

    pub fn result_with_checkpoint(
        self,
        checkpoint: &Checkpoint,
        initial_restore: bool,
    ) -> FormatResult<T> {
        if initial_restore {
            self.af.out.restore_checkpoint(checkpoint);
        }
        self.result_inner(Some(checkpoint))
    }

    fn result_inner(self, checkpoint: Option<&Checkpoint>) -> FormatResult<T> {
        let mut iter = self.strategies.into_iter();
        let first = iter.next().expect("must provide at least one strategy");
        if iter.len() == 0 {
            // avoid creating a checkpoint if there is only one strategy
            return first();
        }
        let new_checkpoint;
        let checkpoint = match checkpoint {
            Some(checkpoint) => checkpoint,
            None => {
                new_checkpoint = self.af.out.checkpoint();
                &new_checkpoint
            }
        };
        let mut result = first();
        for strategy in iter {
            let is_done = match &result {
                Ok(_) => true,
                Err(e) => match e.kind {
                    FormatErrorKind::UnsupportedSyntax => true,
                    FormatErrorKind::Vertical(_) => self.af.constraints().single_line.get(),
                    _ => false,
                },
            };
            if is_done {
                break;
            }
            self.af.out.restore_checkpoint(&checkpoint);
            result = strategy();
        }
        result
    }
}
