#[must_use]
pub trait Guard {}
impl<T> Guard for T {}

pub fn drop_fn(f: impl FnOnce()) -> impl Guard {
    struct Impl<F: FnOnce()>(Option<F>);
    impl<F: FnOnce()> Drop for Impl<F> {
        fn drop(&mut self) {
            self.0.take().unwrap()()
        }
    }
    Impl(Some(f))
}
