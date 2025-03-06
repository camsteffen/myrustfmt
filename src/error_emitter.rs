use std::cell::Cell;
use crate::error::FormatError;
use std::path::PathBuf;

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

    pub fn emit_newline_not_allowed(&self, line: u32, col: u32) {
        self.error_count.set(self.error_count.get() + 1);
        let (line, col) = (line + 1, col + 1);
        eprint!("Newline character not allowed at ");
        match &self.path {
            None => eprintln!("{line}:{col}"),
            Some(path) => eprintln!("{}:{line}:{col}", path.display()),
        }
    }

    pub fn emit_width_exceeded(&self, line_no: u32, line: &str) {
        self.error_count.set(self.error_count.get() + 1);
        eprint!("Max width exceeded at ");
        match &self.path {
            None => eprintln!("line {line_no}"),
            Some(path) => eprintln!("{}:{line_no}", path.display()),
        }
        if cfg!(debug_assertions) {
            eprintln!("line: {line}");
        }
    }

    // todo rename
    #[track_caller]
    pub fn fatal_format_error(&self, e: FormatError, source: &str, pos: usize) -> ! {
        // todo don't panic?
        // todo make it possible to panic inside ErrorEmitter instead?
        panic!(
            "This is a bug :(\n{}",
            e.display(source, pos, self.path.as_deref())
        )
    }
}
