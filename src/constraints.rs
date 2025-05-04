use crate::error::FormatResult;
use crate::num::HPos;
use crate::util::cell_ext::CellExt;
use std::cell::Cell;
use std::num::NonZero;

#[derive(Clone, Debug, PartialEq)]
pub struct Constraints {
    max_width: Cell<HPos>,
    // width limit and max width are very similar in effect, but they are separate values because
    // they may change independently of each other
    width_limit: Cell<Option<WidthLimit>>,
    vertical_shape: Cell<VerticalShape>,
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
    fn end(self) -> HPos {
        match self {
            WidthLimit::SingleLine { end } => end.get(),
            WidthLimit::FirstLine { end, .. } => end.get(),
        }
    }

    pub fn is_applicable(self, at_line: u32) -> bool {
        match self {
            WidthLimit::SingleLine { .. } => true,
            WidthLimit::FirstLine { line, .. } => line == at_line,
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
            vertical_shape: Cell::new(VerticalShape::Unrestricted),
        }
    }

    // basic getters

    pub fn max_width(&self) -> HPos {
        self.max_width.get()
    }

    pub fn width_limit(&self) -> Option<WidthLimit> {
        self.width_limit.get()
    }

    pub fn vertical_shape(&self) -> VerticalShape {
        self.vertical_shape.get()
    }

    // more getters

    pub fn max_width_at(&self, line: u32) -> HPos {
        let Some(width_limit_end) = self.width_limit_end_at(line) else {
            return self.max_width();
        };
        self.max_width().min(width_limit_end)
    }

    fn width_limit_end_at(&self, line: u32) -> Option<HPos> {
        let Some(width_limit) = self.width_limit() else {
            return None;
        };
        match width_limit {
            WidthLimit::SingleLine { end } => Some(end.into()),
            WidthLimit::FirstLine { end, line: l } => (l == line).then_some(end.into()),
        }
    }

    // effects

    pub fn with_width_limit<T>(&self, width_limit: WidthLimit, scope: impl FnOnce() -> T) -> T {
        if matches!(width_limit, WidthLimit::SingleLine { .. }) {
            debug_assert_eq!(self.vertical_shape(), VerticalShape::SingleLine);
        }
        if self
            .width_limit()
            .is_some_and(|current| current.end() <= width_limit.end())
        {
            return scope();
        }
        self.with_replace_width_limit(Some(width_limit), scope)
    }

    pub fn with_replace_max_width<T>(&self, max_width: HPos, scope: impl FnOnce() -> T) -> T {
        self.max_width.with_replaced(max_width, scope)
    }

    /// Replace without regard to the current setting
    pub fn with_replace_vertical_shape<T>(
        &self,
        vertical_shape: VerticalShape,
        scope: impl FnOnce() -> T,
    ) -> T {
        self.vertical_shape.with_replaced(vertical_shape, scope)
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
        if shape == VerticalShape::SingleLine || self.vertical_shape() >= shape {
            return scope();
        }
        self.with_replace_vertical_shape(VerticalShape::SingleLine, scope)
    }

    pub fn has_vertical_shape_if<T>(
        &self,
        condition: bool,
        shape: VerticalShape,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if condition {
            self.has_vertical_shape(shape, scope)
        } else {
            scope()
        }
    }

    pub fn with_replace_width_limit<T>(
        &self,
        width_limit: Option<WidthLimit>,
        scope: impl FnOnce() -> T,
    ) -> T {
        self.width_limit.with_replaced(width_limit, scope)
    }
}
