use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;

/// A Tail squeezes the code before it leftward to make room for itself.
///
/// Used to dynamically specify code that should appear immediately after the primary output
/// of a formatting function. For example, a Tail can be a `;` to be added at the end of an
/// expression. This is to ensure that the code preceding the Tail is formatted in a way that leaves
/// room for the Tail, without overflowing the max width, if possible.
///
/// A Tail is unaware of its width by design, to avoid error-prone math.
///
/// As a general rule, ONLY add a Tail argument to a function if it affects the formatting strategy.
pub struct Tail<'a>(TailImpl<'a>);

enum TailImpl<'a> {
    None,
    Fn(Box<dyn Fn(&AstFormatter) -> FormatResult + 'a>),
    Token(&'static str),
    TokenInsert(&'static str),
    TokenMaybeMissing(&'static str),
}

impl<'a> Tail<'a> {
    pub const fn none() -> &'static Tail<'static> {
        const { &Tail(TailImpl::None) }
    }

    pub fn func(f: impl Fn(&AstFormatter) -> FormatResult + 'a) -> Tail<'a> {
        Tail(TailImpl::Fn(Box::new(f)))
    }

    pub const fn token(token: &'static str) -> Self {
        Tail(TailImpl::Token(token))
    }

    pub const fn token_insert(token: &'static str) -> Self {
        Tail(TailImpl::TokenInsert(token))
    }

    pub const fn token_maybe_missing(token: &'static str) -> Self {
        Tail(TailImpl::TokenMaybeMissing(token))
    }
}

impl AstFormatter {
    // todo audit the following:
    /// N.B. When in doubt, call this function *after* the end of a constraint scope.
    /// For example, if an expression is formatted with a single-line constraint, the tail does not
    /// also need to be single-line.
    pub fn tail(&self, tail: &Tail) -> FormatResult {
        match tail.0 {
            TailImpl::None => Ok(()),
            TailImpl::Fn(ref f) => f(self),
            TailImpl::Token(token) => self.out.token(token),
            TailImpl::TokenInsert(token) => self.out.token_insert(token),
            TailImpl::TokenMaybeMissing(token) => self.out.token_maybe_missing(token),
        }
    }
}
