use crate::ast_formatter::AstFormatter;
use crate::constraints::{MultiLineShape, WidthLimit};
use crate::error::FormatResult;
use crate::util::cell_ext::CellExt;

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
pub struct Tail<'a>(Option<TailImpl<'a>>);

struct TailImpl<'a> {
    func: Box<dyn Fn(&AstFormatter) -> FormatResult + 'a>,
    multi_line: MultiLineShape,
    width_limit: Option<WidthLimit>,
}

// Tail creation
impl AstFormatter {
    pub fn tail_fn<'a>(&self, tail: impl Fn(&AstFormatter) -> FormatResult + 'a) -> Tail<'a> {
        Tail(Some(TailImpl {
            func: Box::new(tail),
            multi_line: self.constraints().multi_line.get(),
            width_limit: self.constraints().width_limit.get(),
        }))
    }

    pub fn tail_token<'a>(&self, token: &'static str) -> Tail<'a> {
        self.tail_fn(|af| af.out.token(token))
    }
}

impl Tail<'_> {
    pub const fn none() -> &'static Tail<'static> {
        const { &Tail(None) }
    }
}

impl AstFormatter {
    pub fn tail(&self, tail: &Tail) -> FormatResult {
        let Some(tail) = &tail.0 else { return Ok(()) };
        self.constraints().multi_line.with_replaced(
            tail.multi_line,
            || self.constraints().width_limit.with_replaced(tail.width_limit, || (tail.func)(self)),
        )
    }
}
