use crate::error::FormatError;
use std::path::PathBuf;

pub struct ErrorEmitter {
    path: Option<PathBuf>,
}

impl ErrorEmitter {
    pub fn new(path: Option<PathBuf>) -> ErrorEmitter {
        ErrorEmitter { path }
    }

    pub fn emit_width_exceeded(&self, line: u32) {
        let place = match &self.path {
            None => format!("line {line}"),
            Some(path) => format!("{}:{line}", path.display()),
        };
        eprintln!("Max width exceeded at {place}");
    }

    // todo rename
    pub fn fatal_format_error(&self, e: FormatError, source: &str, pos: usize) -> ! {
        // todo don't panic?
        panic!(
            "This is a bug :(\n{}",
            e.display(source, pos, self.path.as_deref(),)
        )
    }
}
