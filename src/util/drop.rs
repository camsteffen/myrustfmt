use std::mem::ManuallyDrop;

/// Any type that has side effects when dropped.
// We don't use `impl Drop` because that does not work for compound types containing types that
// implement Drop.
#[must_use]
pub trait Guard {}
impl<T> Guard for T {}

pub fn drop_fn(f: impl FnOnce()) -> impl Guard {
    struct Impl<F: FnOnce()>(ManuallyDrop<F>);
    impl<F: FnOnce()> Drop for Impl<F> {
        fn drop(&mut self) {
            // SAFETY: this field will not be accessed again
            let f = unsafe { ManuallyDrop::take(&mut self.0) };
            f();
        }
    }
    Impl(ManuallyDrop::new(f))
}
