use crate::config::Config;
use crate::error::FormatResult;
use crate::util::cell_ext::CellExt;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaxWidthForLine {
    pub line: u32,
    pub max_width: u32,
}

/// Specifies what kind of multi-line shapes are allowed, if any.
///
/// The MultiLineConstraint is enforced in two ways:
///   1. The SingleLine variant causes an error to be raised upon attempting to write a newline
///      character.
///   2. Other variants are "downgraded" to the SingleLine variant at times when it is known that
///      any newline character would violate the original constraint.
/// 
/// At least the first line of output leading up to a newline must be written to the buffer before
/// raising an error. This makes the implementation simpler by reducing code paths. But more
/// importantly, it allows us to observe the first line of formatted output and know that it would
/// be the same if no constraint were applied.
// todo using SingleLine to measure the width of the first line should ignore trailing line comments
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum MultiLineShape {
    /// No newline characters allowed
    SingleLine,
    /// Allows constructs with curly braces spanning from the first line to the last line
    BlockIndent,
    /// Allows multi-line lists and lists where the last item overflows
    VerticalList,
    /// Allows wrap-indent after the first line, like in long chains
    HangingIndent,
    /// Allows constructs that are indented in multiple places, such as an if-else expression.
    /// (This is really a non-constraint, but is named to clarify the difference from other modes.)
    DisjointIndent,
}

/// An empty Rc used to count the number of open checkpoints.
///
/// The existence of an open checkpoint means that constraint errors are recoverable by restoring
/// the checkpoint and attempting another formatting strategy. Care must be taken to drop
/// checkpoints _before_ the final (non-recoverable) formatting strategy.
///
/// Using an Rc does make the implementation a bit precarious, but it seems less error-prone than
/// having to manually decrement the count at many early return sites.
#[derive(Clone, Debug, Default)]
pub struct CheckpointCounter(Rc<()>);

impl CheckpointCounter {
    pub fn count(&self) -> usize {
        Rc::strong_count(&self.0) - 1
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Constraints {
    /// Things would be much simpler without this
    // todo no Option
    pub max_width: Cell<Option<u32>>,
    /// Used to set the max width for the current line, so it no longer applies after a newline
    /// character is printed
    pub max_width_for_line: Cell<Option<MaxWidthForLine>>,
    // todo is this a constraint?
    /// The number of spaces for the current level of indentation
    pub indent: Cell<u32>,
    pub multi_line: Cell<MultiLineShape>,
}

impl Default for Constraints {
    fn default() -> Self {
        Constraints::new(Config::default().max_width)
    }
}

impl Constraints {
    pub fn new(max_width: u32) -> Constraints {
        Constraints {
            indent: Cell::new(0),
            max_width: Cell::new(Some(max_width)),
            max_width_for_line: Cell::new(None),
            multi_line: Cell::new(MultiLineShape::DisjointIndent),
        }
    }

    pub fn with_multi_line_shape<T>(
        &self,
        constraint: MultiLineShape,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if self.multi_line.get() <= constraint {
            f()
        } else {
            self.multi_line.with_replaced(constraint, f)
        }
    }

    pub fn with_indent_middle(&self, scope: impl Fn() -> FormatResult) -> FormatResult {
        self.with_multi_line_shape(MultiLineShape::HangingIndent, scope)
    }

    pub fn with_no_hanging_indent(&self, scope: impl Fn() -> FormatResult) -> FormatResult {
        self.with_multi_line_shape(MultiLineShape::VerticalList, scope)
    }

    /// Unless the given MultiLineConstraint is applicable, enforce a single-line constraint
    pub fn with_single_line_unless<T>(
        &self,
        shape: MultiLineShape,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if self.multi_line.get() >= shape {
            scope()
        } else {
            self.multi_line
                .with_replaced(MultiLineShape::SingleLine, scope)
        }
    }

    pub fn with_single_line_unless_opt<T>(
        &self,
        shape: Option<MultiLineShape>,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(shape) = shape else {
            return scope();
        };
        self.with_single_line_unless(shape, scope)
    }
}
