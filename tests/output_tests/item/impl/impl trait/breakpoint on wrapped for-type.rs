// test-kind: breakpoint

impl<T> XXX
    for YYY<T>
{
    fn f() {}
}

// :after:

impl<T> XXX
    for YYY<
        T,
    >
{
    fn f() {}
}
