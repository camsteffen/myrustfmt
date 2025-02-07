use crate::error::FormatError;
use std::path::PathBuf;

pub struct ErrorEmitter {
    path: Option<PathBuf>,
}

impl ErrorEmitter {
    pub fn new(path: Option<PathBuf>) -> ErrorEmitter {
        ErrorEmitter { path }
    }

    pub fn emit_width_exceeded(&self, line_no: u32, line: &str) {
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
        panic!(
            "This is a bug :(\n{}",
            e.display(source, pos, self.path.as_deref(),)
        )
    }
}
