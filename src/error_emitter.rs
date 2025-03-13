use std::cell::Cell;
use std::path::PathBuf;
use crate::num::HPos;
use crate::util::cell_ext::{CellExt, CellNumberExt};

#[derive(Debug)]
pub enum Error {
    NewlineNotAllowed { line: u32, col: HPos },
    WidthLimitExceeded { line: u32, line_string: String },
}

pub struct BufferedErrorEmitter {
    /// Buffered errors. Errors are buffered whenever there are any checkpoints.
    buffer: Cell<Vec<Error>>,
    checkpoint_count: Cell<u32>,
    emitter: ErrorEmitter,
}

impl Drop for BufferedErrorEmitter {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            assert_eq!(self.checkpoint_count.get(), 0);
        }
    }
}

pub struct Checkpoint {
    buffer_len: usize,
    index: u32,
}

impl BufferedErrorEmitter {
    pub fn new(emitter: ErrorEmitter) -> BufferedErrorEmitter {
        BufferedErrorEmitter {
            checkpoint_count: Cell::new(0),
            buffer: Cell::new(Vec::new()),
            emitter,
        }
    }

    pub fn error_count(&self) -> u32 {
        let buffer_len = self.buffer.with_taken(|b| b.len());
        self.emitter.error_count() + u32::try_from(buffer_len).unwrap()
    }

    pub fn checkpoint(&self) -> Checkpoint {
        let buffer_len = self.buffer.with_taken(|b| b.len());
        let index = self.checkpoint_count.get();
        self.checkpoint_count.increment();
        Checkpoint { buffer_len, index }
    }

    pub fn commit_checkpoint(&self, checkpoint: Checkpoint) {
        self.assert_last_checkpoint(&checkpoint);
        self.checkpoint_count.decrement();
        if !self.is_buffering() {
            self.flush();
        }
    }

    pub fn restore_checkpoint(&self, checkpoint: &Checkpoint) {
        self.assert_last_checkpoint(checkpoint);
        self.buffer.with_taken(|buffer| buffer.truncate(checkpoint.buffer_len));
    }

    pub fn take_from_checkpoint(&self, checkpoint: &Checkpoint) -> Vec<Error> {
        self.assert_last_checkpoint(checkpoint);
        self.buffer.with_taken(|buffer| buffer.split_off(checkpoint.buffer_len))
    }

    pub fn push_vec(&self, errors: Vec<Error>) {
        if self.is_buffering() {
            self.buffer.with_taken(|buffer| buffer.extend(errors));
        } else {
            errors.into_iter().for_each(|error| self.emit(error));
        }
    }

    // actual errors

    pub fn newline_not_allowed(&self, line: u32, col: HPos) {
        if self.is_buffering() {
            self.buffer.with_taken(|b| b.push(Error::NewlineNotAllowed { line, col }));
        } else {
            self.emitter.newline_not_allowed(line, col);
        }
    }

    pub fn width_exceeded(&self, line: u32, line_str: &str) {
        if self.is_buffering() {
            self.buffer(Error::WidthLimitExceeded {
                line,
                line_string: line_str.to_owned(),
            });
        } else {
            self.emitter.width_exceeded(line, line_str);
        }
    }

    // private

    #[track_caller]
    fn assert_last_checkpoint(&self, checkpoint: &Checkpoint) {
        assert_eq!(checkpoint.index, self.checkpoint_count.get() - 1);
    }

    fn is_buffering(&self) -> bool {
        self.checkpoint_count.get() > 0
    }

    fn buffer(&self, error: Error) {
        self.buffer.with_taken(|b| b.push(error));
    }

    fn emit(&self, error: Error) {
        match error {
            Error::NewlineNotAllowed { line, col } => self.emitter.newline_not_allowed(line, col),
            Error::WidthLimitExceeded { line, line_string } => {
                self.emitter.width_exceeded(line, &line_string)
            }
        }
    }

    fn flush(&self) {
        self.buffer.with_taken(|buffer| {
            buffer.drain(..).for_each(|error| {
                self.emit(error);
            });
        })
    }
}

pub struct ErrorEmitter {
    error_count: Cell<u32>,
    path: Option<PathBuf>,
}

impl ErrorEmitter {
    pub fn new(path: Option<PathBuf>) -> ErrorEmitter {
        ErrorEmitter {
            error_count: Cell::new(0),
            path,
        }
    }

    pub fn error_count(&self) -> u32 {
        self.error_count.get()
    }

    pub fn newline_not_allowed(&self, line: u32, col: HPos) {
        self.error_count.increment();
        let (line, col) = (line + 1, col + 1);
        eprint!("Newline character not allowed at ");
        match &self.path {
            None => eprintln!("{line}:{col}"),
            Some(path) => eprintln!("{}:{line}:{col}", path.display()),
        }
    }

    pub fn width_exceeded(&self, line: u32, line_str: &str) {
        self.error_count.increment();
        eprint!("Max width exceeded at ");
        match &self.path {
            None => eprintln!("line {line}"),
            Some(path) => eprintln!("{}:{line}", path.display()),
        }
        if cfg!(debug_assertions) {
            eprintln!("line: {line_str}");
        }
    }
}
