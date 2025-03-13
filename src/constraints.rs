use crate::error::FormatResult;
use std::cell::Cell;
use std::num::NonZero;
use crate::num::HPos;
use crate::util::cell_ext::CellExt;

#[derive(Clone, Debug, PartialEq)]
pub struct Constraints {
    pub max_width: Cell<HPos>,
    // The width limit behaves a lot like max width, but they are separate values because they may
    // change independently of each other.
    pub width_limit: Cell<Option<WidthLimit>>,
    pub vertical: Cell<VerticalShape>,
}

/// Applies a width limit to a specific scope
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WidthLimit {
    /// Used where a single-line constraint is active
    SingleLine { end: NonZero<HPos> },
    /// Applies a width limit to the first line, then falls out of scope
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

/// Specifies what kind of multi-line shapes are allowed.
/// 
/// Each variant is a superset of all preceding variants.
///
/// It is generally enforced in two ways:
///  1. The SingleLine variant causes an error to be raised upon attempting to write a newline.
///  2. Other variants are "downgraded" to the SingleLine variant at times when it is known that
///     a newline character would violate the original constraint.
/// 
/// A couple of reasons for this approach:
///  * It simplifies the implementation since we can simply "decorate" code paths with what shape it
///    has or requires.
///  * It creates an invariant that we'll always emit the entire first line of output leading up to
///    a newline-not-allowed error. Sometimes this is useful to observe how long the first line of
///    output _would_ be if a more permissive shape were enabled.
// todo using SingleLine to measure the width of the first line should ignore trailing line comments
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub enum VerticalShape {
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

impl Constraints {
    pub fn new(max_width: HPos) -> Constraints {
        Constraints {
            max_width: Cell::new(max_width),
            width_limit: Cell::new(None),
            vertical: Cell::new(VerticalShape::Unrestricted),
        }
    }

    pub fn max_width_at(&self, line: u32) -> HPos {
        let Some(width_limit_end) = self.width_limit_end_at(line) else {
            return self.max_width.get();
        };
        self.max_width.get().min(width_limit_end)
    }

    fn width_limit_end_at(&self, line: u32) -> Option<HPos> {
        let Some(width_limit) = self.width_limit.get() else {
            return None;
        };
        match width_limit {
            WidthLimit::SingleLine { end } => Some(end.into()),
            WidthLimit::FirstLine { end, line: l } => (l == line).then_some(end.into()),
        }
    }
}

// effects
impl Constraints {
    pub fn with_single_line<T>(&self, format: impl FnOnce() -> T) -> T {
        self.vertical.with_replaced(VerticalShape::SingleLine, format)
    }

    pub fn with_single_line_opt<T>(
        &self,
        apply: bool,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if !apply {
            return scope();
        }
        self.with_single_line(scope)
    }

    pub fn with_width_limit<T>(&self, width_limit: WidthLimit, scope: impl FnOnce() -> T) -> T {
        if matches!(width_limit, WidthLimit::SingleLine { .. }) {
            debug_assert_eq!(self.vertical.get(), VerticalShape::SingleLine);
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

    /// Requires the given scope to conform to the given VerticalShape
    pub fn with_vertical_shape_min<T>(&self, shape: VerticalShape, scope: impl FnOnce() -> T) -> T {
        if self.vertical.get() <= shape {
            return scope();
        }
        self.vertical.with_replaced(shape, scope)
    }

    /// Declares that the output in the given scope is known to have the given VerticalShape,
    /// but only if the output actually has multiple lines (or starts with a newline).
    ///
    /// If the given VerticalShape is currently allowed, continues normally.
    /// If not, then the required VerticalShape is set to SingleLine for the given scope.
    pub fn has_vertical_shape<T>(
        &self,
        shape: VerticalShape,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if self.vertical.get() >= shape {
            return scope();
        }
        self.vertical.with_replaced(VerticalShape::SingleLine, scope)
    }

    pub fn has_vertical_shape_unless<T>(
        &self,
        shape: VerticalShape,
        condition: bool,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if condition {
            return scope();
        }
        self.has_vertical_shape(shape, scope)
    }
}
