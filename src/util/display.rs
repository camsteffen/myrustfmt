use std::fmt::{Display, Formatter};

pub fn display_from_fn(fmt: impl Fn(&mut Formatter<'_>) -> std::fmt::Result) -> impl Display {
    struct Impl<F>(F);
    impl<F> Display for Impl<F>
    where
        F: Fn(&mut Formatter<'_>) -> std::fmt::Result,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            self.0(f)
        }
    }
    Impl(fmt)
}
