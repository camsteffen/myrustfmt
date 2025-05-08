use crate::ast_formatter::AstFormatter;
use crate::constraints::{Shape, WidthLimit};
use crate::error::FormatResult;

// Tail is usually passed by reference. We don't put the reference inside the Option because we
// wouldn't be able to have functions that create and return Tail.
pub type Tail<'a, 'b> = &'a Option<TailS<'b>>;

/// A Tail squeezes the code before it leftward to make room for itself.
///
/// Used to dynamically specify code that should appear immediately after the primary output
/// of a formatting function. For example, a Tail can be a `;` to be added at the end of an
/// expression. This is to ensure that the code preceding the Tail is formatted in a way that leaves
/// room for the Tail, without overflowing the max width, if possible.
///
/// A Tail is unaware of its width by design, to avoid error-prone math.
///
/// A Tail captures a snapshot of the Constraints when it is created, and those constraints are
/// restored when the Tail is rendered.
///
/// As a general rule, ONLY add a Tail argument to a function if it affects the formatting strategy.
pub struct TailS<'a> {
    func: Box<dyn Fn(&AstFormatter) -> FormatResult + 'a>,
    // captured constraints
    // todo would it be better to explicitly capture and apply constraints where needed?
    // todo what about RecoverableConstraints?
    width_limit: Option<WidthLimit>,
    shape: Shape,
}

// Tail creation
impl AstFormatter {
    pub fn tail_fn<'a>(
        &self,
        tail: impl Fn(&AstFormatter) -> FormatResult + 'a,
    ) -> Option<TailS<'a>> {
        Some(self.tail_fn_inner(tail))
    }

    pub fn tail_fn_inner<'a>(
        &self,
        tail: impl Fn(&AstFormatter) -> FormatResult + 'a,
    ) -> TailS<'a> {
        TailS {
            func: Box::new(tail),
            width_limit: self.width_limit(),
            shape: self.shape(),
        }
    }

    pub fn tail_token<'a>(&self, token: &'static str) -> Option<TailS<'a>> {
        Some(self.tail_token_inner(token))
    }

    pub fn tail_token_inner<'a>(&self, token: &'static str) -> TailS<'a> {
        self.tail_fn_inner(move |af| af.out.token(token))
    }
}

impl AstFormatter {
    pub fn tail(&self, tail: Tail) -> FormatResult {
        let Some(tail) = tail else { return Ok(()) };
        self.with_replace_shape(tail.shape, || {
            self.with_replace_width_limit(tail.width_limit, || (tail.func)(self))
        })
    }
}
