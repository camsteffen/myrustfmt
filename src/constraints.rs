use crate::config::Config;
use crate::error::FormatResult;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaxWidthForLine {
    pub line: u32,
    pub max_width: u32,
}

/// Specifies what kind of multi-line shapes are allowed, if any.
///
/// The MultiLineConstraint is enforced in two ways:
///   1. The SingleLine variant causes an error to be raised upon attempting to write a newline
///      character.
///   2. Other variants are "downgraded" to the SingleLine variant at times when it is known that
///      any newline character would violate the original constraint.
/// 
/// At least the first line of output leading up to a newline must be written to the buffer before
/// raising an error. This makes the implementation simpler by reducing code paths. But more
/// importantly, it allows us to observe the first line of formatted output and know that it would
/// be the same if no constraint were applied.
// todo using SingleLine to measure the width of the first line should ignore trailing line comments
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum MultiLineShape {
    /// No newline characters allowed
    SingleLine,
    /// Allows constructs with curly braces spanning from the first line to the last line
    BlockIndent,
    /// Allows multi-line lists and lists where the last item overflows
    VerticalList,
    /// Allows wrap-indent after the first line, like in long chains
    HangingIndent,
    /// Allows constructs that are indented in multiple places, such as an if-else expression.
    /// (This is really a non-constraint, but is named to clarify the difference from other modes.)
    DisjointIndent,
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

/// Creates a new Rc for every modification to optimize for cheap clones
#[derive(Default, PartialEq)]
pub struct OwnedConstraints(pub RefCell<Rc<Constraints>>);

impl Deref for OwnedConstraints {
    type Target = RefCell<Rc<Constraints>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl OwnedConstraints {
    pub fn with_modified<T>(
        &self,
        modify: impl FnOnce(&mut Constraints),
        scope: impl FnOnce() -> T,
    ) -> T {
        let mut constraints_ref = self.0.borrow_mut();
        let mut constraints = Constraints::clone(&constraints_ref);
        modify(&mut constraints);
        let old = std::mem::replace(&mut *constraints_ref, Rc::new(constraints));
        drop(constraints_ref);
        let out = scope();
        *self.0.borrow_mut() = old;
        out
    }

    pub fn with_replaced<T>(&self, constraints: Rc<Constraints>, scope: impl FnOnce() -> T) -> T {
        let old = std::mem::replace(&mut *self.0.borrow_mut(), constraints);
        let out = scope();
        *self.0.borrow_mut() = old;
        out
    }

    pub fn with_max_width<T>(&self, max_width: Option<u32>, scope: impl FnOnce() -> T) -> T {
        self.with_modified(|c| c.max_width = max_width, scope)
    }

    pub fn with_max_width_for_line<T>(
        &self,
        max_width: Option<MaxWidthForLine>,
        scope: impl FnOnce() -> T,
    ) -> T {
        self.with_modified(|c| c.max_width_for_line = max_width, scope)
    }

    pub fn with_multi_line_shape_min<T>(
        &self,
        shape: MultiLineShape,
        scope: impl FnOnce() -> T,
    ) -> T {
        if self.borrow().multi_line <= shape {
            return scope();
        }
        self.with_multi_line_shape_replaced(shape, scope)
    }

    pub fn with_multi_line_shape_replaced<T>(
        &self,
        shape: MultiLineShape,
        scope: impl FnOnce() -> T,
    ) -> T {
        self.with_modified(|c| c.multi_line = shape, scope)
    }

    /// Unless the given MultiLineConstraint is applicable, enforce a single-line constraint
    pub fn with_single_line_unless<T>(
        &self,
        shape: MultiLineShape,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if self.borrow().multi_line >= shape {
            scope()
        } else {
            self.with_multi_line_shape_replaced(MultiLineShape::SingleLine, scope)
        }
    }

    pub fn with_single_line_unless_opt<T>(
        &self,
        shape: Option<MultiLineShape>,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        let Some(shape) = shape else {
            return scope();
        };
        self.with_single_line_unless(shape, scope)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Constraints {
    /// Things would be much simpler without this
    // todo no Option
    pub max_width: Option<u32>,
    /// Used to set the max width for the current line, so it no longer applies after a newline
    /// character is printed
    pub max_width_for_line: Option<MaxWidthForLine>,
    pub multi_line: MultiLineShape,
}

impl Default for Constraints {
    fn default() -> Self {
        Constraints::new(Config::default().max_width)
    }
}

impl Constraints {
    pub fn new(max_width: u32) -> Constraints {
        Constraints {
            max_width: Some(max_width),
            max_width_for_line: None,
            multi_line: MultiLineShape::DisjointIndent,
        }
    }
}
