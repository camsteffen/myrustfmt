use std::cell::{Cell, RefCell};

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

impl<T> CellExt<T> for RefCell<T> {
    fn with_replaced<U>(&self, value: T, scope: impl FnOnce() -> U) -> U {
        let prev = self.replace(value);
        let out = scope();
        *self.borrow_mut() = prev;
        out
    }

    fn with_taken<U>(&self, f: impl FnOnce(&mut T) -> U) -> U
    where
        T: Default,
    {
        let mut value = self.take();
        let out = f(&mut value);
        *self.borrow_mut() = value;
        out
    }
}
