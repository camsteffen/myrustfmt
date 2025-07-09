use crate::num::{HSize, VSize};
use crate::util::cell_ext::CellExt;
use std::cell::Cell;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    LineCommentNotAllowed { line: VSize, col: HSize },
    MaxWidthExceeded { line: VSize },
    MultiLineCommentNotAllowed { line: VSize, col: HSize },
    UnsupportedSyntax { line: VSize, col: HSize },
}

pub struct BufferedErrorEmitter {
    /// Buffered errors. Errors are buffered whenever there are any checkpoints.
    buffer: Cell<Vec<Error>>,
    checkpoint_count: Cell<u32>,
    emitter: ErrorEmitter,
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

    pub fn finish(self) -> u32 {
        let Self {
            checkpoint_count,
            buffer,
            emitter,
        } = self;
        assert_eq!(checkpoint_count.get(), 0);
        assert!(buffer.into_inner().is_empty());
        emitter.error_count.get()
    }

    pub fn error_count(&self) -> u32 {
        let buffer_len = self.buffer.with_taken(|b| b.len());
        self.emitter.error_count() + u32::try_from(buffer_len).unwrap()
    }

    pub fn checkpoint(&self) -> Checkpoint {
        let buffer_len = self.buffer.with_taken(|b| b.len());
        let index = self.checkpoint_count.get();
        self.checkpoint_count.update(|n| n + 1);
        Checkpoint { buffer_len, index }
    }

    pub fn commit_checkpoint(&self, checkpoint: Checkpoint) {
        self.assert_last_checkpoint(&checkpoint);
        self.checkpoint_count.update(|n| n - 1);
        if !self.is_buffering() {
            self.flush();
        }
    }

    pub fn restore_checkpoint(&self, checkpoint: &Checkpoint) {
        self.assert_last_checkpoint(checkpoint);
        self.buffer.with_taken(|buffer| {
            buffer.truncate(checkpoint.buffer_len);
        });
    }

    // actual errors

    pub fn line_comment_not_allowed(&self, line: VSize, col: HSize) {
        self.buffer_or_emit(Error::LineCommentNotAllowed { line, col });
    }

    pub fn max_width_exceeded(&self, line: VSize) {
        self.buffer_or_emit(Error::MaxWidthExceeded { line });
    }

    pub fn multi_line_comment_not_allowed(&self, line: VSize, col: HSize) {
        self.buffer_or_emit(Error::MultiLineCommentNotAllowed { line, col });
    }

    pub fn unsupported_syntax(&self, line: VSize, col: HSize) {
        self.buffer_or_emit(Error::UnsupportedSyntax { line, col });
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

    fn buffer_or_emit(&self, error: Error) {
        if self.is_buffering() {
            self.buffer(error);
        } else {
            self.emit(error);
        }
    }

    fn emit(&self, error: Error) {
        match error {
            Error::LineCommentNotAllowed { line, col } => {
                self.emitter.line_comment_not_allowed(line, col)
            }
            Error::MaxWidthExceeded { line } => self.emitter.width_exceeded(line),
            Error::MultiLineCommentNotAllowed { line, col } => {
                self.emitter.multi_line_comment_not_allowed(line, col)
            }
            Error::UnsupportedSyntax { line, col } => self.emitter.unsupported_syntax(line, col),
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

macro_rules! emit {
    ($emitter:expr, $($t:tt)*) => {
        $emitter.error_count.update(|n| n + 1);
        eprintln!($($t)*);
    };
}

impl ErrorEmitter {
    pub fn new(path: Option<PathBuf>) -> ErrorEmitter {
        ErrorEmitter {
            error_count: Cell::new(0),
            path,
        }
    }

    fn error_count(&self) -> u32 {
        self.error_count.get()
    }

    pub fn line_comment_not_allowed(&self, line: VSize, col: HSize) {
        emit!(self, "Line comment not allowed{}", self.at(line, col));
    }

    pub fn multi_line_comment_not_allowed(&self, line: VSize, col: HSize) {
        emit!(self, "Multi-line comment not allowed{}", self.at(line, col));
    }

    pub fn unsupported_syntax(&self, line: VSize, col: HSize) {
        emit!(self, "Unsupported syntax{}", self.at(line, col));
    }

    fn width_exceeded(&self, line: VSize) {
        emit!(self, "Max width exceeded{}", self.at_line(line));
    }

    fn at(&self, line: VSize, col: HSize) -> String {
        let (line, col) = (line + 1, col + 1);
        match &self.path {
            None => format!(" at {line}:{col}"),
            Some(path) => format!(" at {path}:{line}:{col}", path = path.display()),
        }
    }

    fn at_line(&self, line: VSize) -> String {
        let line = line + 1;
        match &self.path {
            None => format!(" at line {line}"),
            Some(path) => format!(" at {}:{line}", path.display()),
        }
    }
}
