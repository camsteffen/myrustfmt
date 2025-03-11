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
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum MultiLineShape {
    /// No newline characters allowed
    SingleLine,
    /// Constructs with curly braces like a block or loop/if/match/etc.
    /// All lines between the first and last lines must be indented (e.g. no if/else).
    /// Does not include struct literals since they are deemed more like a list than a block.
    BlockLike,
    /// Allows multi-line lists including lists where the last item overflows
    VerticalList,
    /// Allows "hanging indent" as in a long chain where lines after the first are indented.
    /// Also allows attributes.
    HangingIndent,
    /// Allows everything else
    Unrestricted,
}

/// Creates a new Rc for every modification to optimize for cheap clones
#[derive(PartialEq)]
pub struct OwnedConstraints(RefCell<ConstraintsGen>);

impl Deref for OwnedConstraints {
    type Target = RefCell<ConstraintsGen>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstraintsGen {
    constraints: Rc<Constraints>,
    #[cfg(debug_assertions)]
    generation: u32,
}

impl Deref for ConstraintsGen {
    type Target = Rc<Constraints>;

    fn deref(&self) -> &Self::Target {
        &self.constraints
    }
}

struct ConstraintsChange<T> {
    data: ConstraintsChangeData<T>,
    #[cfg(debug_assertions)]
    prev_generation: u32,
}

impl<Revert: FnOnce(&mut Constraints)> ConstraintsChange<Revert> {
    fn restore(self, constraints: &OwnedConstraints) {
        let mut c_ref = constraints.0.borrow_mut();
        #[cfg(debug_assertions)]
        assert_eq!(c_ref.generation - 1, self.prev_generation);
        c_ref.generation -= 1;
        match self.data {
            ConstraintsChangeData::Ref(prev) => c_ref.constraints = prev,
            ConstraintsChangeData::Revert(revert) => {
                let constraints = Rc::get_mut(&mut c_ref.constraints).expect(
                    "constraint change expected to have exclusive ownership to revert",
                );
                revert(constraints)
            }
        }
    }
}

enum ConstraintsChangeData<Revert> {
    Ref(Rc<Constraints>),
    Revert(Revert),
}

struct ConstraintsChangeGuard<'a, Revert>
where
    Revert: FnOnce(&mut Constraints),
{
    change: Option<ConstraintsChange<Revert>>,
    constraints: &'a OwnedConstraints,
}

impl<Revert> Drop for ConstraintsChangeGuard<'_, Revert>
where
    Revert: FnOnce(&mut Constraints),
{
    fn drop(&mut self) {
        if !std::thread::panicking() {
            if let Some(change) = self.change.take() {
                change.restore(self.constraints);
            }
        }
    }
}

impl OwnedConstraints {
    pub fn new(constraints: Constraints) -> OwnedConstraints {
        OwnedConstraints(RefCell::new(ConstraintsGen {
            constraints: Rc::new(constraints),
            generation: 0,
        }))
    }

    fn change<T>(&self, get: impl Fn(&mut Constraints) -> &mut T + 'static, value: T) -> impl Drop {
        let mut constraints_ref = self.0.borrow_mut();
        let data = if let Some(constraints) = Rc::get_mut(&mut constraints_ref.constraints) {
            let prev = std::mem::replace(get(constraints), value);
            ConstraintsChangeData::Revert(move |constraints: &mut Constraints| {
                *get(constraints) = prev
            })
        } else {
            let mut constraints = Constraints::clone(&constraints_ref.constraints);
            *get(&mut constraints) = value;
            let prev = std::mem::replace(&mut constraints_ref.constraints, Rc::new(constraints));
            ConstraintsChangeData::Ref(prev)
        };
        let change = ConstraintsChange {
            data,
            prev_generation: constraints_ref.generation,
        };
        #[cfg(debug_assertions)]
        {
            constraints_ref.generation += 1;
        }
        ConstraintsChangeGuard {
            change: Some(change),
            constraints: self,
        }
    }

    fn with_replaced_value<V: 'static, T>(
        &self,
        get: impl Fn(&mut Constraints) -> &mut V + 'static,
        value: V,
        scope: impl FnOnce() -> T,
    ) -> T {
        let _guard = self.change(get, value);
        scope()
    }

    pub fn with_max_width<T>(&self, max_width: Option<u32>, scope: impl FnOnce() -> T) -> T {
        self.with_replaced_value(|c| &mut c.max_width, max_width, scope)
    }

    pub fn with_max_width_for_line<T>(
        &self,
        max_width: Option<MaxWidthForLine>,
        scope: impl FnOnce() -> T,
    ) -> T {
        self.with_replaced_value(|c| &mut c.max_width_for_line, max_width, scope)
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
        self.with_replaced_value(|c| &mut c.multi_line, shape, scope)
    }

    /// Unless the given MultiLineConstraint is applicable, enforce a single-line constraint
    // todo these names suck
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

#[derive(Clone, Debug, PartialEq)]
pub struct Constraints {
    /// Things would be much simpler without this
    // todo no Option
    max_width: Option<u32>,
    /// Used to set the max width for the current line, so it no longer applies after a newline
    /// character is printed. When Some, this takes precedence over max_width.
    max_width_for_line: Option<MaxWidthForLine>,
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
            multi_line: MultiLineShape::Unrestricted,
        }
    }

    pub fn max_width_at(&self, line: u32) -> Option<u32> {
        match self.max_width_for_line {
            Some(max_width_for_line) if max_width_for_line.line == line => {
                Some(max_width_for_line.max_width)
            }
            _ => self.max_width,
        }
    }
}
