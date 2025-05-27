use crate::error::FormatResult;
use crate::num::HSize;
use crate::util::cell_ext::CellExt;
use enumset::{EnumSet, EnumSetType};
use std::cell::Cell;
use std::num::NonZero;

#[derive(Clone, Debug, PartialEq)]
pub struct Constraints {
    // pub in_closure_body: Cell<Option</* first line */ VSize>>,
    // pub in_list_overflow: Cell<Option</* first line */ VSize>>,
    pub disallowed_vstructs: Cell<EnumSet<VStruct>>,
    // max_width and width_limit are very similar in effect, but they need to be separate values for
    // a couple of reasons:
    //  1. width_limit can fall out of scope, and then the max_width is used as a fallback.
    //  2. Sometimes we change max_width as an experiment to simulate starting from a different
    //     horizontal position. The width_limit must be left unchanged for these experiments
    //     since it represents limits that are independent of the global limit.
    /// The global maximum width
    max_width: Cell<HSize>,
    // shape: Cell<Shape>,
    // todo using SingleLine to measure the width of the first line should ignore trailing line comments
    pub single_line: Cell<bool>,
    width_limit: Cell<Option<WidthLimit>>,
}

/// Width limit imposed on a specific node or range as part of formatting logic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WidthLimit {
    /// Used where a single-line constraint is active
    SingleLine { end: NonZero<HSize> },
    /// Applies a width limit to the first line, then falls out of scope
    FirstLine { end: NonZero<HSize>, line: u32 },
}

impl WidthLimit {
    fn end(self) -> HSize {
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

/// Specifies a set of code formatting shapes, used to restrict what formatting strategies may be
/// used in a given context. Each variant is a superset of all preceding variants.
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
#[derive(Debug, EnumSetType)]
pub enum VStruct {
    Closure,
    /// Control flow expressions like if/for/loop/while
    ControlFlow,
    /// `match` expressions
    Match,
    /// Includes lists of all shapes including overflow of the last element.
    /// At a high level, this variant includes shapes that are indented between the first and last
    /// lines.
    List,
    /// Includes "hanging indent" shapes (where lines after the first line are indented) such as
    /// long dot chains or infix chains. Also includes attributes above the node.
    HangingIndent,
    /// Formatting patterns where the code touches the margin one or more times in between the first
    /// and last lines, like an if/else chain or a non-indented dot chain.
    // flat dot chain, range, call, multi-line control flow header, multi-line closure header
    // todo include structs with multi-line headers
    BrokenIndent,
}

impl Constraints {
    pub fn new(max_width: HSize) -> Constraints {
        Constraints {
            disallowed_vstructs: Cell::new(EnumSet::empty()),
            // in_closure_body: Cell::new(None),
            // in_list_overflow: Cell::new(None),
            max_width: Cell::new(max_width),
            width_limit: Cell::new(None),
            single_line: Cell::new(false),
        }
    }

    // basic getters

    // pub fn in_closure_body(&self) -> Option<VSize> {
    //     self.in_closure_body.get()
    // }

    // pub fn in_list_item(&self) -> Option<VSize> {
    //     self.in_list_overflow.get()
    // }

    pub fn max_width(&self) -> HSize {
        self.max_width.get()
    }

    pub fn width_limit(&self) -> Option<WidthLimit> {
        self.width_limit.get()
    }

    // more getters

    pub fn max_width_at(&self, line: u32) -> HSize {
        let Some(width_limit_end) = self.width_limit_end_at(line) else {
            return self.max_width();
        };
        self.max_width().min(width_limit_end)
    }

    fn width_limit_end_at(&self, line: u32) -> Option<HSize> {
        let Some(width_limit) = self.width_limit() else {
            return None;
        };
        match width_limit {
            WidthLimit::SingleLine { end } => Some(end.into()),
            WidthLimit::FirstLine { end, line: l } => (l == line).then_some(end.into()),
        }
    }

    // effects

    pub fn disallow_vstructs(
        &self,
        values: impl Into<EnumSet<VStruct>>,
        scope: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        let values = values.into();
        let prev = self.disallowed_vstructs.get();
        self.disallowed_vstructs.with_replaced(prev | values, scope)
    }

    /// Declares that the output in the given scope has the given Shape.
    ///
    /// If this shape is not allowed, then an error will be raised upon emitting a newline.
    /// (This also means that, if no newline is emitted, there will not be an error.)
    pub fn has_vstruct<T>(
        &self,
        vstruct: VStruct,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if self.single_line.get() || !self.disallowed_vstructs.get().contains(vstruct) {
            return scope();
        }
        self.single_line.with_replaced(true, scope)
    }

    pub fn with_width_limit<T>(&self, width_limit: WidthLimit, scope: impl FnOnce() -> T) -> T {
        if matches!(width_limit, WidthLimit::SingleLine { .. }) {
            debug_assert!(self.single_line.get());
        }
        if self
            .width_limit()
            .is_some_and(|current| current.end() <= width_limit.end())
        {
            return scope();
        }
        self.with_replace_width_limit(Some(width_limit), scope)
    }

    pub fn with_replace_max_width<T>(&self, max_width: HSize, scope: impl FnOnce() -> T) -> T {
        self.max_width.with_replaced(max_width, scope)
    }

    pub fn with_replace_width_limit<T>(
        &self,
        width_limit: Option<WidthLimit>,
        scope: impl FnOnce() -> T,
    ) -> T {
        self.width_limit.with_replaced(width_limit, scope)
    }
}
