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

/// Specifies whether multiple lines may be used to format something, and may disallow certain
/// multi-line formatting strategies.
///
/// The MultiLineConstraint is NOT enforced by changing or short-circuiting the formatting strategy.
/// Instead, it is enforced by trying to write a newline so that the ConstraintWriter raises a
/// NewlineNotAllowed error. The variants in between MultiLine and SingleLine are enforced by
/// "downgrading" to SingleLines at the moment when a newline would definitely violate the
/// higher-level constraint. This makes the implementation simpler by reducing code paths. But more
/// importantly, it allows us to observe formatted output leading up to a newline (error), and know
/// that that code would be no different if no multi-line constraint were applied.
// todo closures?
// todo name variants by what they *allow* ?
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum MultiLineConstraint {
    /// No newline characters allowed (enforced by ConstraintWriter)
    SingleLine,
    /// Same as NoHangingIndent, but also disallow multi-line lists and overflow in lists
    SingleLineLists,
    /// Same as IndentMiddle, but also disallow formats where the last line is indented
    NoHangingIndent,
    /// All lines between the first and last lines must be indented away from the margin.
    /// Expressions ending in curly braces must fit the header before the curly braces in one line.
    IndentMiddle,
    /// No constraint
    MultiLine,
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
    pub multi_line: Cell<MultiLineConstraint>,
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
            multi_line: Cell::new(MultiLineConstraint::MultiLine),
        }
    }

    pub fn requires_indent_middle(&self) -> bool {
        self.multi_line.get() <= MultiLineConstraint::IndentMiddle
    }

    pub fn requires_no_hanging_indent(&self) -> bool {
        self.multi_line.get() <= MultiLineConstraint::NoHangingIndent
    }

    pub fn requires_single_line(&self) -> bool {
        self.multi_line.get() <= MultiLineConstraint::SingleLine
    }

    pub fn with_multi_line_constraint<T>(
        &self,
        constraint: MultiLineConstraint,
        f: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if self.multi_line.get() <= constraint {
            f()
        } else {
            self.multi_line.with_replaced(constraint, f)
        }
    }

    pub fn with_indent_middle(&self, scope: impl Fn() -> FormatResult) -> FormatResult {
        self.with_multi_line_constraint(MultiLineConstraint::IndentMiddle, scope)
    }

    pub fn with_no_hanging_indent(&self, scope: impl Fn() -> FormatResult) -> FormatResult {
        self.with_multi_line_constraint(MultiLineConstraint::NoHangingIndent, scope)
    }

    /// If the given MultiLineConstraint is currently applied (or stricter), then the SingleLine
    /// constraint is applied for the given function.
    // todo rename to classify_newlines or single_line_satisfies ?
    pub fn with_multi_line_constraint_to_single_line<T>(
        &self,
        constraint: MultiLineConstraint,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if self.multi_line.get() > constraint {
            scope()
        } else {
            self.multi_line
                .with_replaced(MultiLineConstraint::SingleLine, scope)
        }
    }

    pub fn with_opt_multi_line_constraint_to_single_line<T>(
        &self,
        constraint: Option<MultiLineConstraint>,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if let Some(constraint) = constraint {
            self.with_multi_line_constraint_to_single_line(constraint, scope)
        } else {
            scope()
        }
    }
}
