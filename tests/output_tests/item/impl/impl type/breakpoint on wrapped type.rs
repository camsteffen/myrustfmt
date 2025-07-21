// test-kind: breakpoint

impl
    XXXXXXXX<T>
{
    fn f() {}
}

// :after:

impl
    XXXXXXXX<
        T,
    >
{
    fn f() {}
}
