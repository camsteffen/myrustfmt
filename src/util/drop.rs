use std::mem::ManuallyDrop;

#[must_use]
pub trait Guard {}
impl<T> Guard for T {}

pub fn drop_fn(f: impl FnOnce()) -> impl Guard {
    struct Impl<F: FnOnce()>(ManuallyDrop<F>);
    impl<F: FnOnce()> Drop for Impl<F> {
        fn drop(&mut self) {
            // SAFETY: this field will not be accessed again
            let f = unsafe {
                ManuallyDrop::take(&mut self.0)
            };
            f();
        }
    }
    Impl(ManuallyDrop::new(f))
}
