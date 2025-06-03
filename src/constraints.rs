use crate::error::{FormatErrorKind, FormatResult};
use crate::num::{HSize, VSize};
use crate::util::cell_ext::CellExt;
use enumset::{EnumSet, EnumSetType};
use std::cell::Cell;
use std::num::NonZero;

#[derive(Clone, Debug, PartialEq)]
pub struct Constraints {
    pub disallowed_vstructs: Cell<EnumSet<VStruct>>,
    // max_width and width_limit are very similar in effect, but they need to be separate values for
    // a couple of reasons:
    //  1. width_limit can fall out of scope, and then the max_width is used as a fallback.
    //  2. Sometimes we change max_width as an experiment to simulate starting from a different
    //     horizontal position. The width_limit must be left unchanged for these experiments
    //     since it represents limits that are independent of the global limit.
    /// The global maximum width
    pub max_width: Cell<HSize>,
    /// When Some, we consider width to be recoverable. This means that if a width limit is
    /// exceeded, we may fall back to another formatting strategy that is known to take less width.
    /// The contained value is the line number.
    pub recover_width: Cell<Option<VSize>>,
    // todo using SingleLine to measure the width of the first line should ignore trailing line comments
    pub single_line: Cell<bool>,
    pub width_limit: Cell<Option<WidthLimit>>,
}

/// Width limit imposed on a specific node or range as part of formatting logic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidthLimit {
    pub end_col: NonZero<HSize>,
    pub line: VSize,
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
            max_width: Cell::new(max_width),
            recover_width: Cell::new(None),
            single_line: Cell::new(false),
            width_limit: Cell::new(None),
        }
    }

    pub fn end_col(&self, line: VSize) -> HSize {
        if let Some(width_limit) = self.width_limit.get() && width_limit.line == line {
            self.max_width.get().min(width_limit.end_col.get())
        } else {
            self.max_width.get()
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
        self.single_line.with_replaced(true, scope).map_err(
            |mut err| {
                if err.kind.is_vertical() {
                    err.kind = FormatErrorKind::VStruct { cause: Box::new(err.kind) };
                }
                err
            },
        )
    }

    pub fn with_width_limit<T>(&self, width_limit: WidthLimit, scope: impl FnOnce() -> T) -> T {
        if self
            .width_limit
            .get()
            .is_some_and(|wl| wl.end_col <= width_limit.end_col)
        {
            return scope();
        }
        self.with_replace_width_limit(Some(width_limit), scope)
    }

    pub fn with_replace_width_limit<T>(
        &self,
        width_limit: Option<WidthLimit>,
        scope: impl FnOnce() -> T,
    ) -> T {
        self.width_limit.with_replaced(width_limit, scope)
    }
}
