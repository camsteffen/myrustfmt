use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;

/// Used to add trailing tokens to a formatted node
/// 
/// * DO accept a Tail argument to make a node narrower to make room for trailing tokens
/// * DON'T accept a Tail argument if it is not used to trigger fallback formats
/// * DON'T pass a Tail argument when the single-line constraint is invariably enabled
#[derive(Clone, Copy)]
pub struct Tail<'a>(TailImpl<'a>);

#[derive(Clone, Copy)]
enum TailImpl<'a> {
    None,
    Const(fn(&AstFormatter) -> FormatResult),
    Dyn(&'a dyn Fn() -> FormatResult),
}

impl<'a> Tail<'a> {
    pub const NONE: Tail<'static> = Tail(TailImpl::None);
    pub const OPEN_BLOCK: Tail<'static> = Tail::new_const(|af| {
        af.out.space()?;
        af.out.token_expect("{")?;
        Ok(())
    });
    pub const SEMICOLON: Tail<'static> = Tail::new_const(|af| af.out.token_expect(";"));

    pub fn new(f: &'a (dyn Fn() -> FormatResult + 'a)) -> Self {
        Tail(TailImpl::Dyn(f))
    }

    pub const fn new_const(f: fn(&AstFormatter) -> FormatResult) -> Self {
        Tail(TailImpl::Const(f))
    }
}

impl AstFormatter {
    pub fn tail(&self, tail: Tail) -> FormatResult {
        match tail.0 {
            TailImpl::None => Ok(()),
            TailImpl::Dyn(f) => f(),
            TailImpl::Const(f) => f(self),
        }
    }
}
