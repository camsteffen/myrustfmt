use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;
use std::rc::Rc;

/// Used to add trailing tokens to a formatted node
///
/// * DO accept a Tail argument to "coerce" a node to leave room for trailing tokens
/// * DON'T accept a Tail argument if it is not used to trigger fallback formats
/// * DON'T pass a Tail argument when the single-line constraint is invariably enabled
#[derive(Clone)]
pub struct Tail(Option<TailImpl>);

#[derive(Clone)]
enum TailImpl {
    And(Rc<(TailImpl, TailImpl)>),
    Token(&'static str),
    TokenMaybeMissing(&'static str),
    TokenMissing(&'static str),
    TokenSkipIfPresent(&'static str),
}

impl Tail {
    pub const fn none() -> &'static Tail {
        const { &Tail(None) }
    }

    pub const fn token(token: &'static str) -> Self {
        Tail(Some(TailImpl::Token(token)))
    }

    pub const fn token_maybe_missing(token: &'static str) -> Self {
        Tail(Some(TailImpl::TokenMaybeMissing(token)))
    }
    
    pub const fn token_missing(token: &'static str) -> Self {
        Tail(Some(TailImpl::TokenMissing(token)))
    }

    pub const fn token_skip_if_present(token: &'static str) -> Self {
        Tail(Some(TailImpl::TokenSkipIfPresent(token)))
    }

    pub fn and(&self, other: &Tail) -> Tail {
        let inner = match (&self.0, &other.0) {
            (None, a) | (a, None) => a.clone(),
            (Some(this), Some(other)) => {
                Some(TailImpl::And(Rc::new((this.clone(), other.clone()))))
            }
        };
        Tail(inner)
    }

    pub fn prefix_token(&self, token: &'static str) -> Self {
        match &self.0 {
            None => Tail(Some(TailImpl::Token(token))),
            Some(after) => Tail(Some(TailImpl::And(Rc::new((
                TailImpl::Token(token),
                after.clone(),
            ))))),
        }
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
            TailImpl::And(and) => {
                let (a, b) = &**and;
                self.tail_inner(a)?;
                self.tail_inner(b)?;
            }
            TailImpl::Token(token) => self.out.token(token)?,
            TailImpl::TokenMaybeMissing(token) => self.out.token_maybe_missing(token)?,
            TailImpl::TokenMissing(token) => self.out.token_missing(token)?,
            TailImpl::TokenSkipIfPresent(token) => self.out.skip_token_if_present(token)?,
        }
        Ok(())
    }
}
