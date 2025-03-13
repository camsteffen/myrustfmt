use crate::error::FormatResult;
use std::cell::RefCell;
use std::num::NonZero;
use std::ops::Deref;
use std::rc::Rc;

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
#[derive(Clone, Debug, PartialEq)]
pub enum WidthLimit {
    /// Applies a width limit where a single-line constraint is active
    SingleLine { end: NonZero<u32> },
    /// Applies a width limit to the first line of some output, then falls out of scope
    FirstLine { end: NonZero<u32>, line: u32 },
}

impl WidthLimit {
    fn end(&self) -> u32 {
        match self {
            WidthLimit::SingleLine { end } => end.get(),
            WidthLimit::FirstLine { end, .. } => end.get(),
        }
    }
}

/// Creates a new Rc for every modification to optimize for cheap clones
#[derive(PartialEq)]
pub struct OwnedConstraints(RefCell<OwnedConstraintsData>);

impl Deref for OwnedConstraints {
    type Target = RefCell<OwnedConstraintsData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OwnedConstraintsData {
    pub max_width: Option<u32>,
    pub scoped_constraints: Rc<Constraints>,
}

impl OwnedConstraints {
    pub fn new(constraints: Constraints, max_width: Option<u32>) -> OwnedConstraints {
        OwnedConstraints(RefCell::new(OwnedConstraintsData {
            scoped_constraints: Rc::new(constraints),
            max_width,
        }))
    }

    fn with_replaced_value<T, U>(
        &self,
        get: impl Fn(&mut Constraints) -> &mut T,
        value: T,
        scope: impl FnOnce() -> U,
    ) -> U {
        let mut constraints_ref = self.0.borrow_mut();
        if let Some(constraints) = Rc::get_mut(&mut constraints_ref.scoped_constraints) {
            let prev = std::mem::replace(get(constraints), value);
            drop(constraints_ref);
            let out = scope();
            let mut c_ref = self.0.borrow_mut();
            let constraints = Rc::get_mut(&mut c_ref.scoped_constraints).expect(
                "constraint change expected to have exclusive ownership to revert",
            );
            *get(constraints) = prev;
            out
        } else {
            let mut constraints = Constraints::clone(&constraints_ref.scoped_constraints);
            *get(&mut constraints) = value;
            let prev = std::mem::replace(
                &mut constraints_ref.scoped_constraints,
                Rc::new(constraints),
            );
            drop(constraints_ref);
            let out = scope();
            self.0.borrow_mut().scoped_constraints = prev;
            out
        }
    }

    pub fn max_width_at(&self, line: u32) -> Option<u32> {
        let ref_ = self.borrow();
        let Some(scoped) = ref_.scoped_constraints.scoped_max_width_at(line) else {
            return ref_.max_width;
        };
        let Some(max_width) = ref_.max_width else {
            return Some(scoped);
        };
        Some(scoped.min(max_width))
    }

    pub fn multi_line(&self) -> MultiLineShape {
        self.borrow().scoped_constraints.multi_line
    }

    pub fn with_global_max_width<T>(&self, max_width: Option<u32>, scope: impl FnOnce() -> T) -> T {
        let prev = std::mem::replace(&mut self.0.borrow_mut().max_width, max_width);
        let out = scope();
        self.0.borrow_mut().max_width = prev;
        out
    }

    pub fn with_no_width_limit<T>(&self, scope: impl FnOnce() -> T) -> T {
        self.with_replaced_value(|c| &mut c.width_limit, None, scope)
    }

    pub fn with_width_limit<T>(&self, width_limit: WidthLimit, scope: impl FnOnce() -> T) -> T {
        if matches!(width_limit, WidthLimit::SingleLine { .. }) {
            debug_assert_eq!(self.multi_line(), MultiLineShape::SingleLine);
        }
        let ignore = {
            let current = &self.0.borrow().scoped_constraints.width_limit;
            current
                .as_ref()
                .is_some_and(|current| current.end() <= width_limit.end())
        };
        if ignore {
            return scope();
        }
        self.with_replaced_value(|c| &mut c.width_limit, Some(width_limit), scope)
    }

    pub fn with_multi_line_shape_min<T>(
        &self,
        shape: MultiLineShape,
        scope: impl FnOnce() -> T,
    ) -> T {
        if self.multi_line() <= shape {
            return scope();
        }
        self.with_multi_line_shape_replaced(shape, scope)
    }

    pub fn with_multi_line_shape_replaced<T>(
        &self,
        shape: MultiLineShape,
        scope: impl FnOnce() -> T,
    ) -> T {
        self.with_replaced_value(|c| &mut c.multi_line, shape, scope)
    }

    /// Unless the given MultiLineConstraint is applicable, enforce a single-line constraint
    // todo these names suck
    pub fn with_single_line_unless<T>(
        &self,
        shape: MultiLineShape,
        scope: impl FnOnce() -> FormatResult<T>,
    ) -> FormatResult<T> {
        if self.multi_line() >= shape {
            scope()
        } else {
            self.with_multi_line_shape_replaced(MultiLineShape::SingleLine, scope)
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
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Constraints {
    pub multi_line: MultiLineShape,
    width_limit: Option<WidthLimit>,
}

impl Constraints {
    pub fn scoped_max_width_at(&self, line: u32) -> Option<u32> {
        let Some(width_limit) = &self.width_limit else {
            return None;
        };
        match *width_limit {
            WidthLimit::SingleLine { end } => Some(end.into()),
            WidthLimit::FirstLine { end, line: l } => (l == line).then_some(end.into()),
        }
    }
}
