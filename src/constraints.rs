use crate::error::{ConstraintErrorKind, FormatResult};
use crate::num::{HSize, VSize};
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
    FirstLine { end: NonZero<HSize>, line: VSize },
}

impl WidthLimit {
    fn end(self) -> HSize {
        match self {
            WidthLimit::SingleLine { end } => end.get(),
            WidthLimit::FirstLine { end, .. } => end.get(),
        }
    }

    pub fn is_applicable(self, at_line: VSize) -> bool {
        match self {
            WidthLimit::SingleLine { .. } => true,
            WidthLimit::FirstLine { line, .. } => line == at_line,
        }
    }
}

/// "Vertical structures". Various formatting shapes that may be disallowed in certain contexts and
/// formatting strategies. It's "vertical" because this describes shapes that span multiple lines.
#[derive(Debug, EnumSetType)]
pub enum VStruct {
    /// Closures when they span multiple lines.
    Closure,
    /// Control flow expressions (if/for/loop/while)
    ControlFlow,
    /// All kinds of lists (e.g. arrays, tuples, call arguments) when they span multiple lines.
    List,
    /// `match` expressions
    Match,
    /// "block indent" means that the first and last lines of the node are not indented, and all
    /// lines in between are indented. This variant describes anything that deviates from that
    /// pattern.
    /// 
    /// Some examples:
    ///  * Nodes with attributes
    ///  * Control flow expressions where the header is multiple lines
    ///  * if/else expressions
    ///  * Multi-line dot chains and infix chains (with or without hanging indentation)
    ///  * Cast expressions with `as <type>` wrapped and indented
    ///  * Closure expressions with arguments on separate lines
    NonBlockIndent,
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

    pub fn max_width(&self) -> HSize {
        self.max_width.get()
    }

    pub fn width_limit(&self) -> Option<WidthLimit> {
        self.width_limit.get()
    }

    // more getters

    pub fn max_width_at(&self, line: VSize) -> HSize {
        let Some(width_limit_end) = self.width_limit_end_at(line) else {
            return self.max_width();
        };
        self.max_width().min(width_limit_end)
    }

    fn width_limit_end_at(&self, line: VSize) -> Option<HSize> {
        let Some(width_limit) = self.width_limit() else {
            return None;
        };
        match width_limit {
            WidthLimit::SingleLine { end } => Some(end.into()),
            WidthLimit::FirstLine { end, line: l } => (l == line).then_some(end.into()),
        }
    }

    // effects

    pub fn allow_vstructs(
        &self,
        values: impl Into<EnumSet<VStruct>>,
        scope: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        let mut next = self.disallowed_vstructs.get();
        next.remove_all(values.into());
        self.disallowed_vstructs.with_replaced(next, scope)
    }

    pub fn disallow_vstructs(
        &self,
        values: impl Into<EnumSet<VStruct>>,
        scope: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        let mut next = self.disallowed_vstructs.get();
        next.insert_all(values.into());
        self.disallowed_vstructs.with_replaced(next, scope)
    }

    /// Declares that the output in the given scope has the given VStruct.
    ///
    /// If this shape is not allowed, then "single line" mode is enabled, so an error will be raised
    /// upon emitting a newline.
    ///
    /// The reason we enable "single line" mode instead of returning an error immediately is because
    /// we sometimes measure the width of the first line of output to inform a decision between
    /// formatting strategies. It also makes this function more flexible by allowing for cases where
    /// the scope fits in one line.
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
