use crate::config::Config;
use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaxWidthForLine {
    pub line: u32,
    pub max_width: u32,
}

enum MultiLineConstraint {
    /// No constraint
    Any,
    /// All lines between the first and last lines must be indented away from the margin
    IndentMiddle,
    /// Same as IndentMiddle, but also disallow multi-line prefix chains and postfix chains
    SingleLineChains,
    /// No multi-line allowed
    SingleLine,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Constraints {
    /// The presence of an open checkpoint indicates that an alternate formatting strategy is
    /// available upstream in case the current code path fails with a constraint error.
    pub open_checkpoint_count: Cell<u32>,
    pub max_width: Cell<Option<u32>>,
    /// Used to set the max width for the current line, so it no longer applies after a newline
    /// character is printed
    pub max_width_for_line: Cell<Option<MaxWidthForLine>>,
    /// If true, no newline characters allowed
    // todo clarify whether it is okay to short circuit strategies when this is true. There is a problem
    //   when we do an experiment with single-line constraint assuming that downstream code will attempt multi-line strategies.
    //   Maybe there should be an "experimental" mode or "no short-circuit" mode
    pub single_line: Cell<bool>,
    // todo is this a constraint?
    /// The number of spaces for the current level of indentation
    pub indent: Cell<u32>,
    /// When true, we say the margin doesn't like to be touched by the expression in question.
    /// If an expression is touching the margin too much, we wrap it with a block to push it away.
    /// More specifically, the expression should only touch the margin in its first and last lines.
    ///
    /// In the example below, the chain expression is not indented, and so is "touching the margin"
    /// for every item in the chain. This makes it harder to scan the arm patterns of the match.
    /// Therefore, match arm bodies are formatted with `touchy_margin` enabled. In the example
    /// below, this would cause the chain to be wrapped with a block.
    ///
    /// ```
    /// match x {
    ///     Some(pattern) => a({
    ///         todo!()
    ///     })
    ///     .multi_line
    ///     .chain,
    ///     _ => {}
    /// }
    /// ```
    // todo rename to indent_middle, or list_margin, or fallback_type=ArmBody?
    //   is the concept here that it is a same line nested scope?
    //   that is, the expression is nested in a match arm or closure body or list item,
    //   so we want the boundary of that outer scope to be clear
    // todo consider splitting off another constraint that disallows "wrap-indent" or just "wrapping"
    //   that is, binary and postfix
    // this constraint is enforced at the AST layer and not ConstraintWriter
    //   1) the semantics are peculiar and feels more naturally understood in AST layer
    //   2) A ConstraintWriter implementation would not be able to fast-fail as quickly.
    pub touchy_margin: Cell<bool>,
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
            single_line: Cell::new(false),
            touchy_margin: Cell::new(false),
        }
    }

    pub fn has_open_checkpoints(&self) -> bool {
        self.open_checkpoint_count.get() > 0
    }
}
