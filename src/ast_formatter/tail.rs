use crate::ast_formatter::AstFormatter;
use crate::error::FormatResult;

/// Used to add trailing tokens to a formatted node
///
/// * DO accept a Tail argument to make a node narrower to make room for trailing tokens
/// * DON'T accept a Tail argument if it is not used to trigger fallback formats
/// * DON'T pass a Tail argument when the single-line constraint is invariably enabled
#[derive(Clone)]
pub struct Tail(Option<TailImpl>);

#[derive(Clone)]
enum TailImpl {
    And(Box<TailImpl>, Box<TailImpl>),
    Const(fn(&AstFormatter) -> FormatResult),
    Token(&'static str),
}

impl Tail {
    pub const NONE: &'static Tail = &Tail(None);
    pub const OPEN_BLOCK: &'static Tail = &Tail::new_const(|af| {
        af.out.space()?;
        af.out.token_expect("{")?;
        Ok(())
    });

    pub const fn new_const(f: fn(&AstFormatter) -> FormatResult) -> Self {
        Tail(Some(TailImpl::Const(f)))
    }

    pub fn token(token: &'static str) -> Self {
        Tail(Some(TailImpl::Token(token)))
    }

    pub fn prefix_token(&self, token: &'static str) -> Self {
        match &self.0 {
            None => Tail(Some(TailImpl::Token(token))),
            Some(after) => Tail(Some(TailImpl::And(
                Box::new(TailImpl::Token(token)),
                Box::new(after.clone()),
            ))),
        }
    }
}

impl AstFormatter {
    pub fn tail(&self, tail: &Tail) -> FormatResult {
        fn do_tail(af: &AstFormatter, tail: &TailImpl) -> FormatResult {
            match tail {
                TailImpl::And(a, b) => {
                    do_tail(af, a)?;
                    do_tail(af, b)?;
                }
                TailImpl::Const(f) => f(af)?,
                TailImpl::Token(token) => af.out.token_expect(token)?,
            }
            Ok(())
        }
        if let Some(tail) = &tail.0 {
            do_tail(self, tail)?;
        }
        Ok(())
    }
}
