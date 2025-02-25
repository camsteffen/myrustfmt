use std::rc::Rc;
use crate::ast_formatter::AstFormatter;
use crate::constraints::Constraints;
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
pub struct Tail<'a>(Option<TailImpl<'a>>);

struct TailImpl<'a> {
    constraints: Rc<Constraints>,
    kind: TailKind<'a>,
}

pub enum TailKind<'a> {
    Fn(Box<dyn Fn(&AstFormatter) -> FormatResult + 'a>),
    Token(&'static str),
    TokenInsert(&'static str),
    TokenMaybeMissing(&'static str),
}

impl AstFormatter {
    pub fn make_tail<'a>(&self, kind: TailKind<'a>) -> Tail<'a> {
        Tail(Some(TailImpl {
            constraints: Rc::clone(&self.constraints().borrow()),
            kind,
        }))
    }

    pub fn tail_fn<'a>(&self, tail: impl Fn(&AstFormatter) -> FormatResult + 'a) -> Tail<'a> {
        self.make_tail(TailKind::Fn(Box::new(tail)))
    }
}

impl<'a> Tail<'a> {
    pub const fn none() -> &'static Tail<'static> {
        const { &Tail(None) }
    }
}

impl AstFormatter {
    // todo audit the following:
    /// N.B. When in doubt, call this function *after* the end of a constraint scope.
    /// For example, if an expression is formatted with a single-line constraint, the tail does not
    /// also need to be single-line.
    pub fn tail(&self, tail: &Tail) -> FormatResult {
        let Some(tail) = &tail.0 else { return Ok(()) };
        self.constraints()
            .with_replaced(Rc::clone(&tail.constraints), || match tail.kind {
                TailKind::Fn(ref f) => f(self),
                TailKind::Token(token) => self.out.token(token),
                TailKind::TokenInsert(token) => self.out.token_insert(token),
                TailKind::TokenMaybeMissing(token) => self.out.token_maybe_missing(token),
            })
    }
}
