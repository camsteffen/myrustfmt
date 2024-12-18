use crate::ast_formatter::AstFormatter;
use crate::source_formatter::FormatResult;

#[derive(Clone, Copy)]
pub struct Tail<'a>(TailImpl<'a>);

#[derive(Clone, Copy)]
enum TailImpl<'a> {
    None,
    Dyn(&'a dyn Fn(&mut AstFormatter<'_>) -> FormatResult),
    Static(fn(&mut AstFormatter<'_>) -> FormatResult),
}

impl<'a> Tail<'a> {
    pub const NONE: Tail<'static> = Tail(TailImpl::None);
    pub const OPEN_BLOCK: Tail<'static> = Tail(TailImpl::Static(|this| {
        this.out.space()?;
        this.out.token_expect("{")?;
        Ok(())
    }));

    pub fn new(f: &'a (dyn Fn(&mut AstFormatter<'_>) -> FormatResult + 'a)) -> Self {
        Tail(TailImpl::Dyn(f))
    }
}

impl AstFormatter<'_> {
    pub fn tail(&mut self, tail: Tail) -> FormatResult {
        match tail.0 {
            TailImpl::None => Ok(()),
            TailImpl::Dyn(f) => f(self),
            TailImpl::Static(f) => f(self),
        }
    }
}
