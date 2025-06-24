use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;
use std::panic::Location;

pub trait FormatResultExt {
    #[allow(unused)]
    fn debug_err(self, af: &AstFormatter) -> Self;

    #[allow(unused)]
    fn debug_err_backtrace(self, af: &AstFormatter) -> Self;
}

impl<T> FormatResultExt for FormatResult<T> {
    #[allow(unused)]
    #[track_caller]
    fn debug_err(self, af: &AstFormatter) -> Self {
        if let Err(e) = &self {
            let location = Location::caller();
            af.out.with_taken_buffer(|buf| {
                eprintln!("[{location}] Error: {:?}\nBuffer:\n{buf}", e.kind);
            });
        }
        self
    }

    #[allow(unused)]
    #[track_caller]
    fn debug_err_backtrace(self, af: &AstFormatter) -> Self {
        if let Err(e) = &self {
            let location = Location::caller();
            af.out.with_taken_buffer(|buf| {
                eprintln!(
                    "[{location}] Error: {:?}\nBuffer:\n{buf}\nBacktrace:\n{}",
                    e.kind,
                    &e.backtrace,
                );
            });
        }
        self
    }
}
