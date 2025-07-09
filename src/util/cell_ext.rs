use crate::util::drop::{Guard, drop_fn};
use std::cell::Cell;

pub trait CellExt<T> {
    /// Updates the value and restores the previous value when the guard is dropped.
    fn map_guard(&self, map: impl FnOnce(T) -> T) -> impl Guard
    where
        T: Copy;

    /// Replaces the value and restores the previous value when the guard is dropped.
    fn replace_guard(&self, value: T) -> impl Guard;

    /// Takes the value, replacing it with the default value, and provides it to the given function.
    /// After the function returns, the value is returned to the Cell.
    fn with_taken<U>(&self, scope: impl FnOnce(&mut T) -> U) -> U
    where
        T: Default;
}

impl<T> CellExt<T> for Cell<T> {
    fn map_guard(&self, map: impl FnOnce(T) -> T) -> impl Guard
    where
        T: Copy,
    {
        self.replace_guard(map(self.get()))
    }

    fn replace_guard(&self, value: T) -> impl Guard {
        let prev = self.replace(value);
        drop_fn(|| self.set(prev))
    }

    fn with_taken<U>(&self, scope: impl FnOnce(&mut T) -> U) -> U
    where
        T: Default,
    {
        let mut value = self.take();
        let out = scope(&mut value);
        self.set(value);
        out
    }
}
