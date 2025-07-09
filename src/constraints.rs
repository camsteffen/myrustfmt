use crate::Recover;
use crate::error::{FormatError, FormatErrorKind, FormatResult, WidthLimitExceededError};
use crate::num::{HSize, VSize};
use crate::util::cell_ext::CellExt;
use crate::util::drop::Guard;
use enumset::{EnumSet, EnumSetType};
#[cfg(debug_assertions)]
use std::backtrace::Backtrace;
use std::cell::Cell;
use std::num::NonZero;
use std::rc::Rc;

pub struct Constraints {
    pub disallowed_vstructs: Cell<VStructSet>,
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
    // todo add function to get width limit only if current line
    // An Rc is needed here so that a Tail may capture a WidthLimit and update its simulate state.
    pub width_limit: Cell<Option<Rc<WidthLimit>>>,
    pub version: Cell<u32>,
}

#[derive(Debug)]
pub struct ConstraintsCheckpoint {
    width_limit_simulate_exceeded: bool,
}

/// Width limit imposed on a specific node or range as part of formatting logic.
pub struct WidthLimit {
    pub end_col: NonZero<HSize>,
    pub line: VSize,
    pub simulate: Option<WidthLimitSimulate>,
}

/// Used to track whether a width limit is exceeded without enforcing the width limit.
///
/// We cannot simply measure the width of the output from a given function because it might include
/// a [`Tail`] where some of the code is not applicable to the width limit.
#[derive(Clone, Debug, Default)]
pub struct WidthLimitSimulate {
    pub exceeded: Cell<bool>,
}

pub type VStructSet = EnumSet<VStruct>;

/// "Vertical structures". Various formatting shapes that may be disallowed in certain contexts and
/// formatting strategies. It's "vertical" because this describes shapes that span multiple lines.
#[derive(Debug, EnumSetType)]
pub enum VStruct {
    /// Blocks when they span multiple lines
    Block,
    /// Closures when they span multiple lines
    Closure,
    /// Control flow expressions (if/for/loop/while)
    ControlFlow,
    // todo use this more?
    Index,
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
            disallowed_vstructs: Cell::new(VStructSet::empty()),
            max_width: Cell::new(max_width),
            recover_width: Cell::new(None),
            single_line: Cell::new(false),
            width_limit: Cell::new(None),
            version: Cell::new(0),
        }
    }

    pub fn checkpoint(&self) -> ConstraintsCheckpoint {
        ConstraintsCheckpoint {
            width_limit_simulate_exceeded: self
                .width_limit()
                .as_ref()
                .and_then(|wl| wl.simulate.as_ref())
                .map_or(false, |WidthLimitSimulate { exceeded }| exceeded.get()),
        }
    }

    pub fn restore_checkpoint(&self, checkpoint: &ConstraintsCheckpoint) {
        let ConstraintsCheckpoint {
            width_limit_simulate_exceeded,
        } = *checkpoint;
        if let Some(width_limit) = self.width_limit()
            && let Some(simulate) = &width_limit.simulate
        {
            simulate.exceeded.set(width_limit_simulate_exceeded);
        }
    }

    pub fn err(&self, kind: FormatErrorKind) -> FormatError {
        FormatError {
            kind,
            #[cfg(debug_assertions)]
            backtrace: Box::new(Backtrace::capture()),
            context_version: self.version.get(),
        }
    }

    pub fn require_remaining_width(
        &self,
        line: VSize,
        col: HSize,
    ) -> Result<HSize, WidthLimitExceededError> {
        let effective_end_col = if let Some(width_limit) = self.width_limit()
            && width_limit.line == line
        {
            if let Some(simulate) = &width_limit.simulate {
                if col > width_limit.end_col.get() {
                    simulate.exceeded.set(true);
                }
                self.max_width.get()
            } else {
                self.max_width.get().min(width_limit.end_col.get())
            }
        } else {
            self.max_width.get()
        };
        effective_end_col
            .checked_sub(col)
            .ok_or(WidthLimitExceededError)
    }

    pub fn width_limit(&self) -> Option<Rc<WidthLimit>> {
        self.width_limit.with_taken(|w| w.as_ref().map(Rc::clone))
    }

    // effects

    pub fn disallow_vstructs(
        &self,
        values: impl Into<VStructSet>,
        recover: &Recover,
        scope: impl FnOnce() -> FormatResult,
    ) -> FormatResult {
        let values = values.into();
        let _guard = self.disallowed_vstructs.map_guard(|set| set | values);
        scope().inspect_err(|e| {
            if let FormatErrorKind::VStruct { vstruct, .. } = e.kind
                && values.contains(vstruct)
            {
                recover.set(true);
            }
        })
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
        // todo can't short circuit if single line mode b/c of `|| for x in y`
        if !self.disallowed_vstructs.get().contains(vstruct) {
            return scope();
        }
        let _guard = self.version.map_guard(|n| n + 1);
        let version = self.version.get();
        let _guard = self.single_line.replace_guard(true);
        scope().map_err(|mut err| {
            if let FormatErrorKind::Vertical(cause) = err.kind
                && err.context_version >= version
            {
                err.kind = FormatErrorKind::VStruct { cause, vstruct };
            }
            err
        })
    }

    pub fn width_limit_guard(&self, width_limit: WidthLimit) -> Option<impl Guard> {
        if let Some(wl) = self.width_limit()
            && wl.line == width_limit.line
            && wl.end_col <= width_limit.end_col
        {
            return None;
        }
        Some(self.width_limit.replace_guard(Some(Rc::new(width_limit))))
    }
}
