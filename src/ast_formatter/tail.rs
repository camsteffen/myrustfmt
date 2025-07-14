use crate::ast_formatter::AstFormatter;
use crate::constraints::{VStructSet, WidthLimit};
use crate::error::FormatResult;
use crate::util::cell_ext::CellExt;
use std::rc::Rc;

pub type Tail<'a, 'b> = Option<&'a TailS<'b>>;

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
    disallowed_vstructs: VStructSet,
    single_line: bool,
    width_limit: Option<Rc<WidthLimit>>,
    constraint_version: u32,
}

// Tail creation
impl AstFormatter {
    pub fn tail_fn<'a>(&self, tail: impl Fn(&AstFormatter) -> FormatResult + 'a) -> TailS<'a> {
        TailS {
            func: Box::new(tail),
            disallowed_vstructs: self.constraints().disallowed_vstructs.get(),
            single_line: self.constraints().single_line.get(),
            width_limit: self.constraints().width_limit(),
            constraint_version: self.constraints().version.get(),
        }
    }

    pub fn tail_token<'a>(&self, token: &'static str) -> TailS<'a> {
        self.tail_fn(move |af| af.out.token(token))
    }
}

impl AstFormatter {
    pub fn tail(&self, tail: Tail) -> FormatResult {
        let Some(tail) = tail else { return Ok(()) };
        let _guard = self.constraints().version.replace_guard(
            tail.constraint_version,
        );
        let _guard = self.constraints().disallowed_vstructs.replace_guard(
            tail.disallowed_vstructs,
        );
        let _guard = self.constraints().single_line.replace_guard(
            tail.single_line,
        );
        let _guard = self.constraints().width_limit.replace_guard(
            tail.width_limit.as_ref().map(Rc::clone),
        );
        (tail.func)(self)
    }
}
