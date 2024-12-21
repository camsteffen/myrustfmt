use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;

/// Used to add trailing tokens to a formatted node, ensuring that
/// the formatting pattern allows room for them.
/// 
/// Formatting functions should only accept a Tail argument if it will be used
/// to fall back to a different format when it doesn't fit.
#[derive(Clone, Copy)]
pub struct Tail<'a>(TailImpl<'a>);

#[derive(Clone, Copy)]
enum TailImpl<'a> {
    None,
    Dyn(&'a dyn Fn() -> FormatResult),
    Static(fn(&AstFormatter) -> FormatResult),
}

impl<'a> Tail<'a> {
    pub const NONE: Tail<'static> = Tail(TailImpl::None);
    pub const OPEN_BLOCK: Tail<'static> = Tail(TailImpl::Static(|this| {
        this.out.space()?;
        this.out.token_expect("{")?;
        Ok(())
    }));

    pub fn new(f: &'a (dyn Fn() -> FormatResult + 'a)) -> Self {
        Tail(TailImpl::Dyn(f))
    }
}

impl AstFormatter {
    pub fn tail(&self, tail: Tail) -> FormatResult {
        match tail.0 {
            TailImpl::None => Ok(()),
            TailImpl::Dyn(f) => f(),
            TailImpl::Static(f) => f(self),
        }
    }
}
