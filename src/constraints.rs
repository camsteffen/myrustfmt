use crate::config::Config;
use crate::error::FormatResult;
use crate::util::cell_ext::CellExt;
use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaxWidthForLine {
    pub line: u32,
    pub max_width: u32,
}

/// Specifies whether multiple lines may be used to format something, and may disallow certain
/// multi-line formatting strategies.
///
/// The MultiLineConstraint is NOT enforced by changing or short-circuiting the formatting strategy.
/// It is ALWAYS enforced by trying to write a newline so that the ConstraintWriter raises a
/// NewlineNotAllowed error. The IndentMiddle and SingleLineChains variants are enforced by
/// "downgrading" to SingleLines at the moment when a newline would definitely violate the
/// higher-level constraint. This makes the implementation simpler by reducing code paths. But more
/// importantly, it allows us to observe formatted output leading up to a newline (error), and know
/// that that code would be no different if no multi-line constraint were applied.
// todo closures?
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum MultiLineConstraint {
    /// No constraint
    MultiLine,
    /// All lines between the first and last lines must be indented away from the margin
    IndentMiddle,
    /// Same as IndentMiddle, but also disallow multi-line prefix chains and postfix chains
    SingleLineChains,
    /// Same as SingleLineChains, but also disallow overflow in multi-item lists
    NoOverflow,
    /// No newline characters allowed (enforced by ConstraintWriter)
    SingleLine,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Constraints {
    /// The presence of an open checkpoint indicates that an alternate formatting strategy is
    /// available upstream in case the current code path fails with a constraint error.
    pub open_checkpoint_count: Cell<u32>,
    /// I mean, this is kinda the whole point
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
            open_checkpoint_count: Cell::new(0),
            indent: Cell::new(0),
            max_width: Cell::new(Some(max_width)),
            max_width_for_line: Cell::new(None),
            multi_line: Cell::new(MultiLineConstraint::MultiLine),
        }
    }

    pub fn has_open_checkpoints(&self) -> bool {
        self.open_checkpoint_count.get() > 0
    }

    pub fn requires_indent_middle(&self) -> bool {
        self.multi_line.get() >= MultiLineConstraint::IndentMiddle
    }

    pub fn requires_single_line_chains(&self) -> bool {
        self.multi_line.get() >= MultiLineConstraint::SingleLineChains
    }

    pub fn requires_single_line(&self) -> bool {
        self.multi_line.get() == MultiLineConstraint::SingleLine
    }

    pub fn with_multi_line_constraint(
        &self,
        constraint: MultiLineConstraint,
        f: impl Fn() -> FormatResult,
    ) -> FormatResult {
        if self.multi_line.get() >= constraint {
            f()
        } else {
            self.multi_line.with_replaced(constraint, f)
        }
    }

    pub fn with_indent_middle(&self, f: impl Fn() -> FormatResult) -> FormatResult {
        self.with_multi_line_constraint(
            MultiLineConstraint::IndentMiddle,
            f,
        )
    }

    pub fn with_single_line_chains(&self, f: impl Fn() -> FormatResult) -> FormatResult {
        self.with_multi_line_constraint(
            MultiLineConstraint::SingleLineChains,
            f,
        )
    }
}
