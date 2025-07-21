// test-kind: breakpoint

impl XXX for YYY
where
    T: Copy,
{
    fn f() {}
}

// :after:

impl XXX
    for YYY
where
    T: Copy,
{
    fn f() {}
}
