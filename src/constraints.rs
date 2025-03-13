use crate::error::FormatResult;
use std::cell::Cell;
use std::num::NonZero;
use crate::num::HPos;
use crate::util::cell_ext::CellExt;

/// Specifies what kind of multi-line shapes are allowed, if any.
/// 
/// Each variant allows all the forms specified in preceding variants.
///
/// It is generally enforced in two ways:
///  1. The SingleLine variant causes an error to be raised upon attempting to write a newline
///     character.
///  2. Other variants are "downgraded" to the SingleLine variant at times when it is known that
///     a newline character would violate the original constraint.
///
/// At least the first line of output leading up to a newline must be written to the buffer before
/// raising an error. This makes the implementation simpler by reducing code paths. But more
/// importantly, it allows us to observe the first line of formatted output and know that it would
/// be the same if no constraint were applied.
// todo using SingleLine to measure the width of the first line should ignore trailing line comments
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub enum MultiLineShape {
    /// No newline characters allowed
    SingleLine,
    /// Generally allows nodes with curly braces like a block or loop/if/match, etc.
    /// All lines between the first and last lines must be indented (e.g. no if/else).
    /// Does not include struct literals since they are counted as lists in this context.
    BlockLike,
    /// Allows lists in any form including overflow.
    /// This should include anything that is indented through the middle lines.
    List,
    /// Allows "hanging indent" such as a wrapped chain where lines after the first are indented.
    /// Also allows attributes.
    HangingIndent,
    /// Allows everything else
    #[default]
    Unrestricted,
}

/// WidthLimit behaves very much like max_width, but they are separate values because both may
/// change, independently of each other.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WidthLimit {
    /// Applies a width limit where a single-line constraint is active
    SingleLine { end: NonZero<HPos> },
    /// Applies a width limit to the first line of some output, then falls out of scope
    FirstLine { end: NonZero<HPos>, line: u32 },
}

impl WidthLimit {
    fn end(&self) -> HPos {
        match self {
            WidthLimit::SingleLine { end } => end.get(),
            WidthLimit::FirstLine { end, .. } => end.get(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Constraints {
    pub max_width: Cell<HPos>,
    pub multi_line: Cell<MultiLineShape>,
    pub width_limit: Cell<Option<WidthLimit>>,
}

impl Constraints {
    pub fn new(max_width: HPos) -> Constraints {
        Constraints {
            max_width: Cell::new(max_width),
            multi_line: Cell::new(MultiLineShape::Unrestricted),
            width_limit: Cell::new(None),
        }
    }

    pub fn max_width_at(&self, line: u32) -> HPos {
        let Some(scoped) = self.scoped_max_width_at(line) else {
            return self.max_width.get();
        };
        self.max_width.get().min(scoped)
    }

    pub fn with_width_limit<T>(&self, width_limit: WidthLimit, scope: impl FnOnce() -> T) -> T {
        if matches!(width_limit, WidthLimit::SingleLine { .. }) {
            debug_assert_eq!(self.multi_line.get(), MultiLineShape::SingleLine);
        }
        if self
            .width_limit
            .get()
            .is_some_and(|current| current.end() <= width_limit.end())
        {
            return scope();
        }
        self.width_limit.with_replaced(Some(width_limit), scope)
    }

    pub fn with_multi_line_shape_min<T>(
        &self,
        shape: MultiLineShape,
        scope: impl FnOnce() -> T,
    ) -> T {
        if self.multi_line.get() <= shape {
            return scope();
        }
        self.multi_line.with_replaced(shape, scope)
    }

    /// Unless the given MultiLineConstraint is applicable, enforce a single-line constraint
    // todo these names suck
    pub fn with_single_line_unless<T>(
        &self,
        shape: MultiLineShape,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if self.multi_line.get() >= shape {
            scope()
        } else {
            let shape1 = MultiLineShape::SingleLine;
            self.multi_line.with_replaced(shape1, scope)
        }
    }

    pub fn with_single_line_unless_or<T>(
        &self,
        shape: MultiLineShape,
        condition: bool,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if condition {
            return scope();
        }
        self.with_single_line_unless(shape, scope)
    }

    pub fn scoped_max_width_at(&self, line: u32) -> Option<HPos> {
        let Some(width_limit) = self.width_limit.get() else {
            return None;
        };
        match width_limit {
            WidthLimit::SingleLine { end } => Some(end.into()),
            WidthLimit::FirstLine { end, line: l } => (l == line).then_some(end.into()),
        }
    }
}
