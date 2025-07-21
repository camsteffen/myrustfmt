// test-kind: breakpoint

impl<T>
    XXXXXXXXX<T>
    for YYY
{
    fn f() {}
}

// :after:

impl<T>
    XXXXXXXXX<
        T,
    > for YYY
{
    fn f() {}
}
