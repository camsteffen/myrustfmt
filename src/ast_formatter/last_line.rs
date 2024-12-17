use crate::ast_formatter::AstFormatter;
use crate::source_formatter::FormatResult;

pub struct Tail(Option<Box<dyn Fn(&mut AstFormatter<'_>) -> FormatResult>>);

impl Tail {
    pub const NONE: Tail = Tail(None);

    pub fn new(f: impl Fn(&mut AstFormatter<'_>) -> FormatResult + 'static) -> Self {
        Tail(Some(Box::new(f)))
    }
}

impl AstFormatter<'_> {
    pub fn tail(&mut self, tail: &Tail) -> FormatResult {
        if let Some(f) = &tail.0 {
            f(self)?;
        }
        Ok(())
    }
}
