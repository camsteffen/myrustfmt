use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;

/// Used to add trailing tokens to a formatted node
///
/// * DO accept a Tail argument to "coerce" a node to leave room for trailing tokens
/// * DON'T accept a Tail argument if it is not used to trigger fallback formats
/// * DON'T pass a Tail argument when the single-line constraint is invariably enabled
pub struct Tail<'a>(Option<TailImpl<'a>>);

enum TailImpl<'a> {
    Fn(Box<dyn Fn(&AstFormatter) -> FormatResult + 'a>),
    Token(&'static str),
    TokenInsert(&'static str),
    TokenMaybeMissing(&'static str),
}

impl<'a> Tail<'a> {
    pub const fn none() -> &'static Tail<'static> {
        const { &Tail(None) }
    }

    pub fn func(f: impl Fn(&AstFormatter) -> FormatResult + 'a) -> Tail<'a> {
        Tail(Some(TailImpl::Fn(Box::new(f))))
    }

    pub const fn token(token: &'static str) -> Self {
        Tail(Some(TailImpl::Token(token)))
    }

    pub const fn token_insert(token: &'static str) -> Self {
        Tail(Some(TailImpl::TokenInsert(token)))
    }

    pub const fn token_maybe_missing(token: &'static str) -> Self {
        Tail(Some(TailImpl::TokenMaybeMissing(token)))
    }
}

impl AstFormatter {
    pub fn tail(&self, tail: &Tail) -> FormatResult {
        if let Some(tail) = &tail.0 {
            self.tail_inner(tail)?;
        }
        Ok(())
    }

    fn tail_inner(&self, tail: &TailImpl) -> FormatResult {
        match tail {
            TailImpl::Fn(f) => f(self)?,
            TailImpl::Token(token) => self.out.token(token)?,
            TailImpl::TokenInsert(token) => self.out.token_insert(token)?,
            TailImpl::TokenMaybeMissing(token) => self.out.token_maybe_missing(token)?,
        }
        Ok(())
    }
}
