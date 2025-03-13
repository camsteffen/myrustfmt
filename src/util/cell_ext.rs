use std::cell::Cell;

pub trait CellExt<T> {
    fn with_replaced<U>(&self, value: T, scope: impl FnOnce() -> U) -> U;
    fn with_taken<U>(&self, scope: impl FnOnce(&mut T) -> U) -> U
    where
        T: Default;
}

impl<T> CellExt<T> for Cell<T> {
    fn with_replaced<U>(&self, value: T, scope: impl FnOnce() -> U) -> U {
        let prev = self.replace(value);
        let out = scope();
        self.set(prev);
        out
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

pub trait CellNumberExt {
    fn decrement(&self);
    fn increment(&self);
}

impl CellNumberExt for Cell<u32> {
    fn decrement(&self) {
        self.set(self.get() - 1);
    }

    fn increment(&self) {
        self.set(self.get() + 1);
    }
}
