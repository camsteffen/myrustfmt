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
}
