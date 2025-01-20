use std::cell::Cell;

pub trait CellExt<T> {
    fn with_replaced<U>(&self, value: T, f: impl FnOnce() -> U) -> U;
}

impl<T> CellExt<T> for Cell<T> {
    #[inline]
    fn with_replaced<U>(&self, value: T, f: impl FnOnce() -> U) -> U {
        let prev = self.replace(value);
        let out = f();
        self.set(prev);
        out
    }
}