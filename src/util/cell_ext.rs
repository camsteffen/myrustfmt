use std::cell::Cell;

pub trait CellExt<T> {
    fn with_replaced<U>(&self, value: T, f: impl FnOnce() -> U) -> U;
    fn with_taken<U>(&self, f: impl FnOnce(&mut T) -> U) -> U
    where
        T: Default;
}

impl<T> CellExt<T> for Cell<T> {
    fn with_replaced<U>(&self, value: T, f: impl FnOnce() -> U) -> U {
        let prev = self.replace(value);
        let out = f();
        self.set(prev);
        out
    }

    fn with_taken<U>(&self, f: impl FnOnce(&mut T) -> U) -> U
    where
        T: Default,
    {
        let mut value = self.take();
        let out = f(&mut value);
        self.set(value);
        out
    }
}
